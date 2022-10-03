//! Event handlers for matrix events.
#![allow(clippy::unused_async)] // Matrix handlers are async

use std::sync::Arc;

use clap::Parser;
use color_eyre::{
	eyre::{bail, eyre},
	Result,
};
use matrix_sdk::{
	event_handler::Ctx,
	room::Room,
	ruma::events::room::{
		member::{MembershipState, StrippedRoomMemberEvent, SyncRoomMemberEvent},
		message::{OriginalSyncRoomMessageEvent, RoomMessageEventContent},
	},
	Client,
};

use crate::{
	commands::{parse_arguments, Command},
	database::Databases,
	settings::Settings,
};

/// Matrix room message event handler.
pub async fn on_room_message(
	event: OriginalSyncRoomMessageEvent,
	room: Room,
	client: Client,
	config: Ctx<Arc<Settings>>,
	db: Ctx<Databases>,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender == own_id {
		return Ok(());
	}

	let room = match room {
		Room::Joined(room) => room,
		_ => bail!("Received message from not-joined room"),
	};

	// Ignore messages from before joining.
	let joined_ts = room
		.get_member_no_sync(own_id)
		.await?
		.ok_or_else(|| eyre!("Couldn't get own join event"))?
		.event()
		.origin_server_ts()
		.ok_or_else(|| eyre!("Own join event does not have timestamp"))?;
	if event.origin_server_ts < joined_ts {
		return Ok(());
	}

	let msg = event.content.body();
	tracing::trace!("{}: {msg}", event.sender);

	// Check if there is a command we need to react on
	if let Some(arguments) = msg.strip_prefix('!') {
		let mut arguments = parse_arguments(arguments);
		arguments.insert(0, String::from("!"));
		match Command::try_parse_from(arguments) {
			Ok(mut command) => {
				command
					.execute(
						&config,
						&db,
						&client,
						&room,
						&event.into_full_event(room.room_id().to_owned()),
					)
					.await?;
			}
			Err(error) => {
				let message = RoomMessageEventContent::text_plain(error.to_string())
					.make_reply_to(&event.into_full_event(room.room_id().to_owned()));
				room.send(message, None).await?;
			}
		}
	}

	Ok(())
}

/// Matrix invite event handler.
pub async fn on_invite_event(
	event: StrippedRoomMemberEvent,
	room: Room,
	client: Client,
	config: Ctx<Arc<Settings>>,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender == own_id {
		return Ok(());
	}

	if event.content.membership != MembershipState::Invite {
		return Ok(());
	}

	let invited = &event.state_key;
	if invited != own_id {
		return Ok(());
	}

	if let Room::Invited(room) = room {
		let room_name = room.name().unwrap_or_else(|| room.room_id().to_string());
		tracing::debug!("Received invite for room {room_name}");

		if config.access.admins.contains(&event.sender) {
			tracing::info!("Joining room {room_name}");
			room.accept_invitation().await?;
		} else {
			tracing::info!("Rejecting invitation to {room_name} from {}", event.sender);
			room.reject_invitation().await?;
		}
	}
	Ok(())
}

/// Matrix room member event handler.
pub async fn on_room_membership_event(
	event: SyncRoomMemberEvent,
	room: Room,
	client: Client,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender() == own_id {
		return Ok(());
	}

	let room = match room {
		Room::Joined(joined) => joined,
		_ => return Ok(()),
	};

	#[allow(clippy::single_match)] // More to come?
	match event.membership() {
		MembershipState::Leave | MembershipState::Ban => {
			// Leave if nobody in the room anymore
			let members = room.joined_user_ids().await?;
			if members.len() <= 1 {
				tracing::info!(
					"Leaving empty room {} ({})",
					room.display_name().await?,
					room.room_id()
				);
				room.leave().await?;
			}
		}
		_ => {}
	}
	Ok(())
}
