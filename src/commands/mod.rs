//! Bot command module.

mod leave;
mod remind;

use clap::Parser;
use color_eyre::Result;
use matrix_sdk::{
	async_trait, room::Joined, ruma::events::room::message::OriginalRoomMessageEvent, Client,
};

use self::{leave::Leave, remind::Remind};
use crate::settings::Settings;

/// The trait every command implements. This is used for executing the command.
#[async_trait]
trait BotCommand {
	/// Execute the command.
	async fn execute<'a>(&self, context: Context<'a>) -> Result<()>;
}

/// The command the bot should execute.
#[derive(Debug, Parser)]
#[clap(name = "Matrix-Bot", author = "FlixCoder", version, about)]
pub enum Command {
	/// Leave the room.
	Leave(Leave),
	/// Remind someone of something, i.e. sends a message at the specified point
	/// in time.
	Remind(Remind),
}

impl Command {
	/// View the command as a trait object.
	fn as_bot_command(&self) -> &(dyn BotCommand + Send + Sync) {
		match self {
			Command::Leave(cmd) => cmd,
			Command::Remind(cmd) => cmd,
		}
	}

	/// Execute the command.
	pub async fn execute(
		&self,
		config: &Settings,
		client: &Client,
		room: &Joined,
		event: &OriginalRoomMessageEvent,
	) -> Result<()> {
		self.as_bot_command().execute(Context { config, client, room, event }).await
	}
}

/// Command context
#[allow(dead_code)] // Context will be used later.
struct Context<'a> {
	/// Configuration
	pub config: &'a Settings,
	/// Matrix SDK Client
	pub client: &'a Client,
	/// Joined room
	pub room: &'a Joined,
	/// Original message event
	pub event: &'a OriginalRoomMessageEvent,
}

/// Parse arguments in a message by splitting it on spaces. This keeps into
/// account quotes for giving arguments that include spaces.
#[allow(clippy::collapsible_else_if)] // more readable
pub fn parse_arguments(message: &str) -> Vec<String> {
	let mut arguments = Vec::new();
	let mut current_arg = String::new();
	let mut current_seperator = None;

	for arg in message.split(' ') {
		if let Some(cur_sep) = current_seperator {
			current_arg.push(' ');
			if let Some(stripped) = arg.strip_suffix(cur_sep) {
				current_arg.push_str(stripped);
				current_seperator = None;
				arguments.push(current_arg.clone());
				current_arg.clear();
			} else {
				current_arg.push_str(arg);
			}
		} else {
			if let Some(stripped) = arg.strip_prefix('\'') {
				if let Some(completely_stripped) = stripped.strip_suffix('\'') {
					arguments.push(completely_stripped.to_owned());
				} else {
					current_seperator = Some('\'');
					current_arg.push_str(stripped);
				}
			} else if let Some(stripped) = arg.strip_prefix('"') {
				if let Some(completely_stripped) = stripped.strip_suffix('"') {
					arguments.push(completely_stripped.to_owned());
				} else {
					current_seperator = Some('"');
					current_arg.push_str(stripped);
				}
			} else {
				arguments.push(arg.to_owned());
			}
		}
	}

	arguments.retain(|arg| !arg.is_empty());
	arguments
}

#[cfg(test)]
mod tests;
