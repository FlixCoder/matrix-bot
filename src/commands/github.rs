//! Github notifications configuration to receive Github notificiations.

use clap::{Args, Subcommand};
use color_eyre::Result;
use matrix_sdk::{async_trait, ruma::events::room::message::RoomMessageEventContent};

use super::{BotCommand, Context};
use crate::{clients, database::GithubSubscription};

/// Github command.
#[derive(Debug, Args)]
pub struct Github {
	/// Github command to execute.
	#[clap(subcommand)]
	command: SubCommand,
}

/// Which Github sub-command to execute.
#[derive(Debug, Subcommand)]
enum SubCommand {
	/// List active Github notification subscriptions for users.
	List,
	/// Clear active Github notification subscriptions.
	Clear,
	/// Enable new Github notification subscription.
	Enable {
		/// Github login username.
		username: String,
		/// Github API token. Get one from https://github.com/settings/tokens.
		token: String,
	},
	/// Disable Github notification subscription.
	Disable {
		/// Github login username.
		username: String,
	},
}

#[async_trait]
impl BotCommand for Github {
	async fn execute<'a>(&mut self, context: Context<'a>) -> Result<()> {
		let is_mod = context.config.access.admins.contains(&context.event.sender)
			|| context.config.access.mods.contains(&context.event.sender);
		if !is_mod {
			tracing::trace!("Person not allowed to edit Github notification settings!");
			return Ok(());
		}

		match &self.command {
			SubCommand::List => {
				let subscriptions =
					GithubSubscription::for_room(context.room.room_id(), &context.db.state).await?;
				let formatted_subscriptions = subscriptions
					.into_values()
					.map(|doc| format!("- {}", doc.contents.user))
					.collect::<Vec<_>>();
				if formatted_subscriptions.is_empty() {
					let msg = RoomMessageEventContent::text_plain(
						"Currently, there are no Github subscriptions.",
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
					GithubSubscription::for_room(context.room.room_id(), &context.db.state)
						.await?
						.into_values()
				{
					subscription.delete_async(&context.db.state).await?;
				}

				let success_msg = RoomMessageEventContent::text_plain(
					"Successfully cleared Github subscriptions.",
				)
				.make_reply_to(context.event);
				context.room.send(success_msg, None).await?;
			}

			SubCommand::Enable { username, token } => {
				let client = clients::github::Github::new(username.clone(), token.clone());
				if client.test_token().await.is_ok() {
					let subscription = GithubSubscription::new(
						context.room.room_id().to_owned(),
						username.clone(),
						token.clone(),
					);
					subscription.insert(&context.db.state).await?;

					let success_msg = RoomMessageEventContent::text_plain(
						"Successfully enabled Github subscription.",
					)
					.make_reply_to(context.event);
					context.room.send(success_msg, None).await?;
				} else {
					let failure_msg = RoomMessageEventContent::text_plain("Token is invalid.")
						.make_reply_to(context.event);
					context.room.send(failure_msg, None).await?;
				}
			}

			SubCommand::Disable { username } => {
				if let Some(subscription) =
					GithubSubscription::find(context.room.room_id(), username, &context.db.state)
						.await?
				{
					subscription.delete_async(&context.db.state).await?;

					let success_msg = RoomMessageEventContent::text_plain(
						"Successfully disabled Github subscription.",
					)
					.make_reply_to(context.event);
					context.room.send(success_msg, None).await?;
				} else {
					let failure_msg =
						RoomMessageEventContent::text_plain("Github subscription not found.")
							.make_reply_to(context.event);
					context.room.send(failure_msg, None).await?;
				}
			}
		}
		Ok(())
	}
}
