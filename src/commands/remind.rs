//! The remind command.

use std::time::{Duration, SystemTime};

use bonsaimq::JobRegister;
use clap::Args;
use color_eyre::eyre::eyre;
use matrix_sdk::{async_trait, ruma::OwnedUserId};

use super::{BotCommand, Context};
use crate::jobs::{remind::RemindInput, JobRegistry};

/// Remind command.
#[derive(Debug, Args)]
pub struct Remind {
	/// Who to remind (MXID) or reminds yourself if not given.
	#[clap(value_parser, short, long)]
	who: Option<OwnedUserId>,
	/// Duration to wait until reminding (e.g "5:30" for remind in 5 hours and
	/// 30 minutes).
	#[clap(value_parser = parse_when)]
	when: Duration,
	/// Reminder message
	#[clap(value_parser)]
	message: String,
}

/// Parse "when" string into a [`Duration`].
fn parse_when(s: &str) -> Result<Duration, String> {
	match s.split_once(':') {
		Some((hours, minutes)) => {
			let hours: u32 = hours.parse().map_err(|_| format!("`{hours}` is not a number!"))?;
			let minutes: u32 =
				minutes.parse().map_err(|_| format!("`{minutes}` is not a number!"))?;
			let secs: u64 = u64::from(hours) * 60 * 60 + u64::from(minutes) * 60;
			Ok(Duration::from_secs(secs))
		}
		None => {
			let minutes: u32 = s
				.parse()
				.map_err(|_| format!("`{s}` is neither a number of minutes, nor e.g. '5:30'!"))?;
			let secs: u64 = u64::from(minutes) * 60;
			Ok(Duration::from_secs(secs))
		}
	}
}

#[async_trait]
impl BotCommand for Remind {
	async fn execute<'a>(&mut self, context: Context<'a>) -> color_eyre::Result<()> {
		let who = if let Some(user_id) = self.who.take() {
			let is_mod = context.config.access.admins.contains(&context.event.sender)
				|| context.config.access.mods.contains(&context.event.sender);
			if !is_mod {
				tracing::trace!("Person not allowed to remind others!");
				return Ok(());
			}
			user_id
		} else {
			context.event.sender.clone()
		};

		let room_id = context.room.room_id().to_owned();

		let message_time = context
			.event
			.origin_server_ts
			.to_system_time()
			.ok_or_else(|| eyre!("Could not get SystemTime of message timestamp"))?;
		let processing_delay = SystemTime::now().duration_since(message_time)?;
		let delay = self.when.saturating_sub(processing_delay);

		let remind_input = RemindInput { who, room_id, message: self.message.clone() };
		JobRegistry::Remind
			.builder()
			.delay(delay)
			.payload_json(remind_input)?
			.spawn(context.db)
			.await?;

		tracing::trace!("Scheduled reminder message.");
		Ok(())
	}
}
