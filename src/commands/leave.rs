//! The leave command.

use clap::Args;
use matrix_sdk::async_trait;

use super::{BotCommand, Context};
use crate::matrix::ClientExt;

/// Leave command.
#[derive(Debug, Args)]
pub struct Leave;

#[async_trait]
impl BotCommand for Leave {
	async fn execute<'a>(&mut self, context: Context<'a>) -> color_eyre::Result<()> {
		if context.config.access.admins.contains(&context.event.sender)
			|| context.config.access.mods.contains(&context.event.sender)
		{
			context.client.leave_room_by_id_no_wait(context.room.room_id()).await?;
		}
		Ok(())
	}
}
