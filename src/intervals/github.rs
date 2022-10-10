//! Github notification subscription interval handler.

use std::collections::HashMap;

use bonsaidb::core::schema::SerializedCollection;
use color_eyre::Result;
use matrix_sdk::{
	room::Joined,
	ruma::{events::room::message::RoomMessageEventContent, OwnedRoomId},
	Client,
};
use time::OffsetDateTime;

use crate::{
	clients::github::{Github, Notification},
	database::{Databases, GithubSubscription},
};

/// State for the github interval.
#[derive(Debug, Default)]
pub struct IntervalState {
	/// Cache of github clients, so that rate limiting is not reached.
	clients: HashMap<(OwnedRoomId, String), Github>,
}

impl IntervalState {
	/// Get or create the client for the room-user pair.
	pub fn get_client(
		&mut self,
		room: OwnedRoomId,
		user: String,
		token: String,
	) -> Result<&mut Github> {
		let new_client = Github::new(user.clone(), token.clone())?;
		let client = self.clients.entry((room, user)).or_insert(new_client);
		client.set_token(token);
		Ok(client)
	}
}

/// Interval function to be called every time the interval fires.
pub async fn interval(db: &Databases, client: &Client, state: &mut IntervalState) -> Result<()> {
	tracing::debug!("Running Github interval..");

	let subscriptions = GithubSubscription::all_async(&db.state).await?;
	for mut subscription in subscriptions {
		if let Some(room) = client.get_joined_room(&subscription.contents.room) {
			let github_client = state.get_client(
				subscription.contents.room.clone(),
				subscription.contents.user.clone(),
				subscription.contents.token.clone(),
			)?;
			if !github_client.next_request_allowed() {
				continue;
			}

			let now = OffsetDateTime::now_utc();
			let notifications =
				github_client.notifications(subscription.contents.latest_update).await?;
			send_notification_messages(&room, &notifications, github_client).await?;

			subscription.contents.latest_update = now;
			subscription.update_async(&db.state).await?;
		} else {
			subscription.delete_async(&db.state).await?;
		}
	}
	Ok(())
}

/// Send messages for the notifications into the room.
async fn send_notification_messages(
	room: &Joined,
	notifications: &[Notification],
	client: &Github,
) -> Result<()> {
	for notification in notifications {
		let (html, body) = render_notification(client, notification).await?;
		let message = if room.is_direct() {
			RoomMessageEventContent::text_html(body, html)
		} else {
			RoomMessageEventContent::notice_html(body, html)
		};
		room.send(message, None).await?;
	}
	Ok(())
}

/// Render a notification as body and html message.
async fn render_notification(
	client: &Github,
	notification: &Notification,
) -> Result<(String, String)> {
	let mut html = String::new();
	let mut body = String::new();

	html.push_str(&format!(
		"<a href={}>{}</a><br>\n",
		notification.repository.html_url, notification.repository.full_name
	));
	body.push_str(&format!("{}\n", notification.repository.full_name));

	if let Some(url) = notification.subject.latest_comment_url.clone() {
		let comment = client.get_thread_comment_from(url).await?;
		let html_url = comment.html_url;

		html.push_str(&format!(
			"<a href=\"{}\"><b>{}: {} ({})</b></a><br>\n",
			html_url, notification.subject.r#type, notification.subject.title, notification.reason
		));
		body.push_str(&format!(
			"{}: {} ({})\n",
			notification.subject.r#type, notification.subject.title, notification.reason
		));

		html.push_str(&format!("{}: {}<br>\n", comment.user.login, comment.body));
		body.push_str(&format!("{}: {}\n", comment.user.login, comment.body));
	} else {
		html.push_str(&format!(
			"<b>{}: {} ({})</b><br>\n",
			notification.subject.r#type, notification.subject.title, notification.reason
		));
		body.push_str(&format!(
			"{}: {} ({})\n",
			notification.subject.r#type, notification.subject.title, notification.reason
		));
	}

	let url = "https://github.com/notifications";
	html.push_str(&format!("<a href=\"{}\">{}</a>", url, "See notifications"));
	body.push_str(url);

	Ok((html, body))
}
