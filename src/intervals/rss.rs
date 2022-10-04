//! RSS interval.

use bonsaidb::core::schema::SerializedCollection;
use color_eyre::Result;
use feed_rs::model::{Entry, Feed};
use matrix_sdk::{room::Joined, ruma::events::room::message::RoomMessageEventContent, Client};
use time::OffsetDateTime;

use crate::database::{Databases, RssSubscription};

/// Interval function to be called every time the interval fires.
pub async fn interval(db: &Databases, client: &Client) -> Result<()> {
	tracing::debug!("Running RSS interval..");
	let http_client = reqwest::Client::new();

	let rss_subs = RssSubscription::all_async(&db.state).await?;
	for mut subscription in rss_subs {
		if let Some(room) = client.get_joined_room(&subscription.contents.room) {
			let feed_bytes =
				http_client.get(subscription.contents.url.clone()).send().await?.bytes().await?;
			let feed = feed_rs::parser::parse(feed_bytes.as_ref())?;

			send_feed_messages(&room, &feed, &subscription.contents.latest_update).await?;

			subscription.contents.latest_update = get_latest_entry(&feed)?;
			subscription.update_async(&db.state).await?;
		} else {
			subscription.delete_async(&db.state).await?;
		}
	}
	Ok(())
}

/// Send out messages for new feed entries into the room.
async fn send_feed_messages(
	room: &Joined,
	feed: &Feed,
	latest_update: &OffsetDateTime,
) -> Result<()> {
	let new_entries = feed.entries.iter().filter(|entry| {
		entry
			.published
			.as_ref()
			.or(entry.updated.as_ref())
			.map_or(false, |dtm| dtm.timestamp() > latest_update.unix_timestamp())
	});

	for entry in new_entries {
		let (message, body) = render_entry(entry);
		let message = RoomMessageEventContent::text_html(body, message);
		room.send(message, None).await?;
	}
	Ok(())
}

/// Render an entry as HTML and raw message.
fn render_entry(entry: &Entry) -> (String, String) {
	let mut message = String::new();
	let mut body = String::new();

	if let Some(title) = &entry.title {
		message.push_str(&format!("<b>{}</b><br>\n", title.content));
		body.push_str(&format!("{}\n", title.content));
	}

	if let Some(summary) = &entry.summary {
		message.push_str(&format!("{}<br>\n", summary.content));
		body.push_str(&format!("{}\n", summary.content));
	}

	for link in &entry.links {
		message.push_str(&format!(
			"<a href=\"{}\">{}</a><br>\n",
			link.href,
			link.title.as_ref().unwrap_or(&link.href)
		));
		body.push_str(&format!("{}\n", link.href));
	}

	(message, body)
}

/// Extract latest entry time from feed.
fn get_latest_entry(feed: &Feed) -> Result<OffsetDateTime> {
	let latest_time = feed
		.entries
		.iter()
		.filter_map(|entry| entry.published.as_ref().or(entry.updated.as_ref()))
		.fold(0, |maximum, dtm| dtm.timestamp().max(maximum));

	let latest_time = OffsetDateTime::from_unix_timestamp(latest_time)?;
	Ok(latest_time)
}
