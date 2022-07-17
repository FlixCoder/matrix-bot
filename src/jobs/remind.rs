//! Remind job.

use std::sync::Arc;

use bonsaimq::CurrentJob;
use color_eyre::{eyre::eyre, Result};
use matrix_sdk::{
	ruma::{events::room::message::RoomMessageEventContent, OwnedRoomId, OwnedUserId},
	Client,
};
use serde::{Deserialize, Serialize};

use crate::settings::Settings;

/// The job's input.
#[derive(Debug, Serialize, Deserialize)]
pub struct RemindInput {
	/// Who to remind.
	pub who: OwnedUserId,
	/// Origin room ID.
	pub room_id: OwnedRoomId,
	/// Reminder message,
	pub message: String,
}

/// Job to remind people of something, outer job error handler.
#[tracing::instrument(level = "debug", skip_all)]
pub async fn job_remind(mut job: CurrentJob) -> Result<()> {
	if let Err(err) = remind(&mut job).await {
		tracing::error!("Cancelling reminder job: {err}");
		job.complete().await?;
		return Err(err);
	}
	Ok(())
}

/// Remind someone of something, inner job.
async fn remind(job: &mut CurrentJob) -> Result<()> {
	let _config: Arc<Settings> =
		job.context().ok_or_else(|| eyre!("Expected settings in context"))?;
	let client: Client = job.context().ok_or_else(|| eyre!("Expected matrix client in context"))?;
	let input: RemindInput = job.payload_json().ok_or_else(|| eyre!("Expected job input"))??;

	tracing::trace!("Sending reminder..",);

	let room =
		client.get_joined_room(&input.room_id).ok_or_else(|| eyre!("Room not in joined rooms"))?;
	let who_name = room
		.get_member_no_sync(&input.who)
		.await?
		.and_then(|who| who.display_name().map(ToOwned::to_owned))
		.unwrap_or_else(|| input.who.localpart().to_owned());

	let message = RoomMessageEventContent::text_html(
		format!("@{}: {}", who_name, input.message),
		format!(
			"<a href=\"https://matrix.to/#/{}\">@{}</a>: {}",
			input.who, who_name, input.message
		),
	);
	room.send(message, None).await?;

	job.complete().await?;
	Ok(())
}
