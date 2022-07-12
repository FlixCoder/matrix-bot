//! Workaround for bugged matrix sdk methods.

use color_eyre::{eyre::eyre, Result};
use matrix_sdk::{
	room::Invited,
	ruma::{events::room::member::MembershipState, OwnedUserId},
};

/// Get the inviter ID for the invite.
pub async fn invite_get_sender(invited: &Invited) -> Result<Option<OwnedUserId>> {
	let user_id = invited.own_user_id();
	let invitee = invited
		.get_member_no_sync(user_id)
		.await?
		.ok_or_else(|| eyre!("Member event not found even though we are invited"))?;
	let event = invitee.event();
	if *event.membership() == MembershipState::Invite {
		let inviter_id = event.sender();
		Ok(Some(inviter_id.to_owned()))
	} else {
		Ok(None)
	}
}
