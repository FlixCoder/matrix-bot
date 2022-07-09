//! Event handlers for matrix events.

use matrix_sdk::{room::Room, ruma::events::room::message::OriginalSyncRoomMessageEvent};

/// Matrix event handler
pub async fn on_room_message(event: OriginalSyncRoomMessageEvent, room: Room) {
	let room = room
		.display_name()
		.await
		.ok()
		.map(|display_name| display_name.to_string())
		.or_else(|| room.name())
		.unwrap_or_default();
	let user = event.sender;
	let msg = event.content.body();
	tracing::trace!("[{room}] {user}: {msg}");
}
