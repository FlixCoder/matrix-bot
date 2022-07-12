//! The leave command.

use clap::Args;
use matrix_sdk::async_trait;

use super::{BotCommand, Context};

/// Leave command.
#[derive(Debug, Args)]
pub struct Leave;

#[async_trait]
impl BotCommand for Leave {
	async fn execute<'a>(&self, context: Context<'a>) -> color_eyre::Result<()> {
		if context.config.access.admins.contains(&context.event.sender)
			|| context.config.access.mods.contains(&context.event.sender)
		{
			context.room.leave().await?;
		}
		Ok(())
	}
}
