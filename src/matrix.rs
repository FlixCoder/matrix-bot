//! Matrix helper functions.

use std::collections::HashSet;

use matrix_sdk::RoomMember;

/// Get the number of unique User-IDs in a room from a list of active members.
pub fn get_unique_members(members: &[RoomMember]) -> usize {
	let user_ids: HashSet<_> = members.iter().map(|member| member.user_id()).collect();
	user_ids.len()
}
