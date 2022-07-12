//! The remind command.

use clap::Args;
use matrix_sdk::async_trait;

use super::{BotCommand, Context};

/// Remind command.
#[derive(Debug, Args)]
pub struct Remind {
	/// Who to remind ("me" or MXID)
	#[clap(value_parser)]
	who: String, // TODO: better type
	/// When to remind (e.g "5h 30m")
	#[clap(value_parser)]
	when: String, // TODO: better type
	/// Reminder message
	#[clap(value_parser)]
	message: String,
}

#[async_trait]
impl BotCommand for Remind {
	async fn execute<'a>(&self, _context: Context<'a>) -> color_eyre::Result<()> {
		// TODO
		Ok(())
	}
}
