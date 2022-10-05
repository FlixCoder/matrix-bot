//! The remind command.

use std::time::Duration;

use bonsaimq::JobRegister;
use clap::Args;
use matrix_sdk::{
	async_trait,
	ruma::{events::room::message::RoomMessageEventContent, OwnedUserId},
};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

use super::{BotCommand, Context};
use crate::jobs::{remind::RemindInput, JobRegistry};

/// Remind command.
#[derive(Debug, Args)]
pub struct Remind {
	/// Who to remind (MXID) or reminds yourself if not given.
	#[arg(short, long)]
	who: Option<OwnedUserId>,
	/// When to remind. Can be either a duration to wait until reminding (e.g
	/// "5:30" for remind in 5 hours and 30 minutes) or a specific date-time
	/// when it should happen in RFC3339 format.
	#[arg(value_parser = parse_when)]
	when: OffsetDateTime,
	/// Reminder message.
	message: String,
}

/// Parse "when" string into a specific date-time to execute the reminder.
fn parse_when(s: &str) -> Result<OffsetDateTime, String> {
	if let Ok(when) = OffsetDateTime::parse(s, &Rfc3339) {
		Ok(when)
	} else {
		let now = OffsetDateTime::now_utc();
		let when_duration = parse_when_duration(s)?;
		Ok(now + when_duration)
	}
}

/// Parse the when string as a [`Duration`] in the format of `%h:%m` or just
/// `%m`.
fn parse_when_duration(s: &str) -> Result<Duration, String> {
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

		let delay = Duration::try_from(self.when - OffsetDateTime::now_utc()).unwrap_or_default();
		let room_id = context.room.room_id().to_owned();
		let remind_input = RemindInput { who, room_id, message: self.message.clone() };

		JobRegistry::Remind
			.builder()
			.delay(delay)
			.payload_json(remind_input)?
			.spawn(&context.db.jobs)
			.await?;

		tracing::trace!("Scheduled reminder message.");
		let scheduled_msg = RoomMessageEventContent::text_plain("Successfully scheduled reminder.")
			.make_reply_to(context.event);
		context.room.send(scheduled_msg, None).await?;

		Ok(())
	}
}
