//! RSS feed configuration to receive and notify of e.g. news via RSS.

use clap::{Args, Subcommand};
use color_eyre::Result;
use matrix_sdk::{async_trait, ruma::events::room::message::RoomMessageEventContent};
use url::Url;

use super::{BotCommand, Context};
use crate::database::RssSubscription;

/// RSS command.
#[derive(Debug, Args)]
pub struct Rss {
	/// RSS command to execute.
	#[clap(subcommand)]
	command: SubCommand,
}

/// Which RSS sub-command to execute.
#[derive(Debug, Subcommand)]
enum SubCommand {
	/// List active RSS feeds.
	List,
	/// Clear active RSS feeds.
	Clear,
	/// Enable new RSS feed.
	Enable {
		/// Full feed URL.
		url: Url,
	},
	/// Disable RSS feed.
	Disable {
		/// Full feed URL.
		url: Url,
	},
}

#[async_trait]
impl BotCommand for Rss {
	async fn execute<'a>(&mut self, context: Context<'a>) -> Result<()> {
		let is_mod = context.config.access.admins.contains(&context.event.sender)
			|| context.config.access.mods.contains(&context.event.sender);
		if !is_mod {
			tracing::trace!("Person not allowed to edit RSS settings!");
			return Ok(());
		}

		match &self.command {
			SubCommand::List => {
				let subscriptions =
					RssSubscription::for_room(context.room.room_id(), &context.db.state).await?;
				let formatted_subscriptions = subscriptions
					.into_values()
					.map(|doc| format!("- {}", doc.contents.url))
					.collect::<Vec<_>>();
				if formatted_subscriptions.is_empty() {
					let msg = RoomMessageEventContent::text_plain(
						"Currently, there are no RSS subscriptions.",
					)
					.make_reply_to(context.event);
					context.room.send(msg, None).await?;
				} else {
					let msg =
						RoomMessageEventContent::text_markdown(formatted_subscriptions.join("\n"))
							.make_reply_to(context.event);
					context.room.send(msg, None).await?;
				}
			}

			SubCommand::Clear => {
				for subscription in
					RssSubscription::for_room(context.room.room_id(), &context.db.state)
						.await?
						.into_values()
				{
					subscription.delete_async(&context.db.state).await?;
				}

				let success_msg =
					RoomMessageEventContent::text_plain("Successfully cleared RSS subscriptions.")
						.make_reply_to(context.event);
				context.room.send(success_msg, None).await?;
			}

			SubCommand::Enable { url } => {
				if test_feed_url(url.clone()).await.is_ok() {
					let subscription =
						RssSubscription::new(context.room.room_id().to_owned(), url.clone());
					subscription.insert(&context.db.state).await?;

					let success_msg = RoomMessageEventContent::text_plain(
						"Successfully enabled RSS subscription.",
					)
					.make_reply_to(context.event);
					context.room.send(success_msg, None).await?;
				} else {
					let failure_msg =
						RoomMessageEventContent::text_plain("URL is not a valid RSS stream.")
							.make_reply_to(context.event);
					context.room.send(failure_msg, None).await?;
				}
			}

			SubCommand::Disable { url } => {
				if let Some(subscription) =
					RssSubscription::find(context.room.room_id(), url, &context.db.state).await?
				{
					subscription.delete_async(&context.db.state).await?;

					let success_msg = RoomMessageEventContent::text_plain(
						"Successfully disabled RSS subscription.",
					)
					.make_reply_to(context.event);
					context.room.send(success_msg, None).await?;
				} else {
					let failure_msg =
						RoomMessageEventContent::text_plain("RSS subscription not found.")
							.make_reply_to(context.event);
					context.room.send(failure_msg, None).await?;
				}
			}
		}
		Ok(())
	}
}

/// Test a URL whether it gives a parsable RSS feed.
async fn test_feed_url(url: Url) -> Result<()> {
	let bytes = reqwest::get(url).await?.bytes().await?;
	let _feed = feed_rs::parser::parse(bytes.as_ref())?;
	Ok(())
}
