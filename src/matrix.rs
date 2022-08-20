//! Matrix helper functions.

use std::collections::HashSet;

use matrix_sdk::{
	async_trait,
	room::Invited,
	ruma::{
		api::client::membership::{join_room_by_id, leave_room},
		RoomId,
	},
	Client, Result, RoomMember,
};

/// Get the number of unique User-IDs in a room from a list of active members.
pub fn get_unique_members(members: &[RoomMember]) -> usize {
	let user_ids: HashSet<_> = members.iter().map(|member| member.user_id()).collect();
	user_ids.len()
}

/// Extended matrix client functionality.
#[async_trait]
pub trait ClientExt {
	/// Join a room without waiting for receiving the event in sync.
	async fn join_room_by_id_no_wait(&self, room_id: &RoomId) -> Result<()>;
	/// Leave a room without waiting for receiving the event in sync.
	async fn leave_room_by_id_no_wait(&self, room_id: &RoomId) -> Result<()>;
}

#[async_trait]
impl ClientExt for Client {
	#[inline]
	async fn join_room_by_id_no_wait(&self, room_id: &RoomId) -> Result<()> {
		let request = join_room_by_id::v3::Request::new(room_id);
		self.send(request, None).await?;
		Ok(())
	}

	#[inline]
	async fn leave_room_by_id_no_wait(&self, room_id: &RoomId) -> Result<()> {
		let request = leave_room::v3::Request::new(room_id);
		self.send(request, None).await?;
		Ok(())
	}
}

/// Accept the invitation without waiting for receiving the event in sync.
#[inline]
pub async fn accept_invitation_no_wait(client: &Client, room: &Invited) -> Result<()> {
	client.join_room_by_id_no_wait(room.room_id()).await
}

/// Reject the invitation without waiting for receiving the event in sync.
#[inline]
pub async fn reject_invitation_no_wait(client: &Client, room: &Invited) -> Result<()> {
	client.leave_room_by_id_no_wait(room.room_id()).await
}
