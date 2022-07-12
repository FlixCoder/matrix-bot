//! Event handlers for matrix events.
#![allow(clippy::unused_async)] // Matrix handlers are async

use color_eyre::{eyre::eyre, Result};
use matrix_sdk::{
	event_handler::Ctx,
	room::Room,
	ruma::events::room::{
		member::{MembershipState, StrippedRoomMemberEvent, SyncRoomMemberEvent},
		message::OriginalSyncRoomMessageEvent,
	},
	Client, RoomType,
};

use crate::{matrix::get_unique_members, settings::Settings};

/// Matrix room message event handler, handling the error of the actual inner
/// handler.
pub async fn on_room_message(
	event: OriginalSyncRoomMessageEvent,
	room: Room,
	client: Client,
	config: Ctx<Settings>,
) {
	if let Err(err) = on_room_message_inner(event, room, client, config).await {
		tracing::error!("Error in on_room_message handler: {err}");
	}
}

/// Actual inner room message handler.
pub async fn on_room_message_inner(
	event: OriginalSyncRoomMessageEvent,
	room: Room,
	client: Client,
	_config: Ctx<Settings>,
) -> Result<()> {
	let room_name = room
		.display_name()
		.await
		.ok()
		.map(|display_name| display_name.to_string())
		.or_else(|| room.name())
		.unwrap_or_default();

	let user = client
		.get_joined_room(room.room_id())
		.ok_or_else(|| eyre!("Got room event for not-joined room"))?
		.get_member_no_sync(&event.sender)
		.await?
		.ok_or_else(|| eyre!("Got room event from not-joined room member"))?;
	let user_name = user.display_name().unwrap_or_else(|| user.user_id().as_str());

	let msg = event.content.body();

	tracing::trace!("[{room_name}] {user_name}: {msg}");

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
