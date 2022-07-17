//! Event handlers for matrix events.
#![allow(clippy::unused_async)] // Matrix handlers are async

use bonsaidb::local::AsyncDatabase;
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
	Client, RoomType,
};

use crate::{
	commands::{parse_arguments, Command},
	matrix::get_unique_members,
	settings::Settings,
};

/// Matrix room message event handler, handling the error of the actual inner
/// handler.
pub async fn on_room_message(
	event: OriginalSyncRoomMessageEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
	db: Ctx<AsyncDatabase>,
) {
	if let Err(err) = on_room_message_inner(event, room, client, config, db).await {
		tracing::error!("Error in on_room_message handler: {err}");
	}
}

/// Actual inner room message handler.
pub async fn on_room_message_inner(
	event: OriginalSyncRoomMessageEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
	db: Ctx<AsyncDatabase>,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender == own_id {
		return Ok(());
	}

	let room = match room {
		Room::Joined(room) => room,
		_ => bail!("Received message from not-joined room"),
	};

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
				let message = RoomMessageEventContent::text_reply_plain(
					error,
					&event.into_full_event(room.room_id().to_owned()),
				);
				room.send(message, None).await?;
			}
		}
	}

	Ok(())
}

/// Matrix invite event handler, handling the error of the actual inner handler.
pub async fn on_invite_event(
	event: StrippedRoomMemberEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
) {
	if let Err(err) = on_invite_inner(event, room, client, config).await {
		tracing::error!("Error in on_invite event handler: {err}");
	}
}

/// Actual inner invite event handler.
async fn on_invite_inner(
	event: StrippedRoomMemberEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender == own_id {
		return Ok(());
	}

	match event.content.membership {
		MembershipState::Invite if room.room_type() == RoomType::Invited => {
			let invited = &event.state_key;
			if invited != own_id {
				return Ok(());
			}

			let room_name = room.name().unwrap_or_else(|| room.room_id().to_string());
			tracing::debug!("Received invite for room {room_name}");

			let invite = client
				.get_invited_room(room.room_id())
				.ok_or_else(|| eyre!("Got invite for room which isn't listed in client"))?;

			if config.access.admins.contains(&event.sender) {
				tracing::info!("Joining room {room_name}");
				invite.accept_invitation().await?;
			} else {
				tracing::info!("Rejecting invitation to {room_name} from {}", event.sender);
				invite.reject_invitation().await?;
			}
		}
		_ => {}
	}
	Ok(())
}

/// Matrix room member event handler, handling the error of the actual inner
/// handler.
pub async fn on_room_membership_event(
	event: SyncRoomMemberEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
) {
	if let Err(err) = on_room_membership_inner(event, room, client, config).await {
		tracing::error!("Error in on_invite event handler: {err}");
	}
}

/// Actual inner room membership event handler.
async fn on_room_membership_inner(
	event: SyncRoomMemberEvent,
	room: Room,
	client: Client,
	_config: Ctx<Settings>,
) -> Result<()> {
	let own_id = client.user_id().ok_or_else(|| eyre!("Couldn't get own user ID"))?;
	if event.sender() == own_id {
		return Ok(());
	}

	#[allow(clippy::single_match)] // More to come?
	match event.membership() {
		MembershipState::Leave => {
			// Leave if nobody in the room anymore
			let members = room.active_members_no_sync().await?;
			if get_unique_members(&members) <= 1 {
				tracing::info!("Leaving room {} ({})", room.display_name().await?, room.room_id());
				client
					.get_joined_room(room.room_id())
					.ok_or_else(|| eyre!("Got leave room event for not-joined room"))?
					.leave()
					.await?;
			}
		}
		_ => {}
	}
	Ok(())
}
