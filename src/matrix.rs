//! Matrix helper functions.

use color_eyre::{eyre::eyre, Result as EyreResult};
use matrix_sdk::{
	async_trait, ruma::events::room::message::RoomMessageEventContent, Client, Result, Session,
};

/// Session store key for access token.
const SESSION_ACCESS_TOKEN: &str = "SESSION_ACCESS_TOKEN";
/// Session store key for refresh token.
const SESSION_REFRESH_TOKEN: &str = "SESSION_REFRESH_TOKEN";
/// Session store key for user ID.
const SESSION_USER_ID: &str = "SESSION_USER_ID";
/// Session store key for device ID.
const SESSION_DEVICE_ID: &str = "SESSION_DEVICE_ID";

/// Extended matrix client functionality.
#[async_trait]
pub trait ClientExt {
	/// Leave empty rooms.
	async fn leave_empty_rooms(&self) -> Result<()>;
	/// Save the current session in the state store.
	async fn save_session(&self) -> EyreResult<()>;
	/// Restore login based on session in the state store. Returns whether the
	/// was session was restored.
	async fn restore_session(&self) -> EyreResult<bool>;
}

#[async_trait]
impl ClientExt for Client {
	async fn leave_empty_rooms(&self) -> Result<()> {
		tracing::debug!("Leaving empty rooms..");
		for room in self.joined_rooms() {
			let members = room.joined_user_ids().await?;
			if members.len() <= 1 {
				tracing::info!("Leaving room {} ({})", room.display_name().await?, room.room_id());
				room.leave().await?;
			}
		}
		Ok(())
	}

	async fn save_session(&self) -> EyreResult<()> {
		let session = self.session().ok_or_else(|| eyre!("No session available to save."))?;

		self.store()
			.set_custom_value(
				SESSION_ACCESS_TOKEN.as_bytes(),
				serde_json::to_vec(&session.access_token)?,
			)
			.await?;

		self.store()
			.set_custom_value(
				SESSION_REFRESH_TOKEN.as_bytes(),
				serde_json::to_vec(&session.refresh_token)?,
			)
			.await?;

		self.store()
			.set_custom_value(SESSION_USER_ID.as_bytes(), serde_json::to_vec(&session.user_id)?)
			.await?;

		self.store()
			.set_custom_value(SESSION_DEVICE_ID.as_bytes(), serde_json::to_vec(&session.device_id)?)
			.await?;

		Ok(())
	}

	async fn restore_session(&self) -> EyreResult<bool> {
		let access_token = if let Some(data) =
			self.store().get_custom_value(SESSION_ACCESS_TOKEN.as_bytes()).await?
		{
			serde_json::from_slice(&data)?
		} else {
			return Ok(false);
		};

		let refresh_token = if let Some(data) =
			self.store().get_custom_value(SESSION_REFRESH_TOKEN.as_bytes()).await?
		{
			serde_json::from_slice(&data)?
		} else {
			return Ok(false);
		};

		let user_id =
			if let Some(data) = self.store().get_custom_value(SESSION_USER_ID.as_bytes()).await? {
				serde_json::from_slice(&data)?
			} else {
				return Ok(false);
			};

		let device_id = if let Some(data) =
			self.store().get_custom_value(SESSION_DEVICE_ID.as_bytes()).await?
		{
			serde_json::from_slice(&data)?
		} else {
			return Ok(false);
		};

		let session = Session { access_token, refresh_token, user_id, device_id };
		self.restore_login(session).await?;

		Ok(true)
	}
}

/// Create a matrix message, but generate escaped HTML for plain text as well as
/// the body.
pub fn plain_message(body: String) -> RoomMessageEventContent {
	let html = body
		.replace('<', "&lt;")
		.replace('>', "&gt;")
		.replace('&', "&amp;")
		.replace('\n', "<br>\n");
	RoomMessageEventContent::text_html(body, html)
}
