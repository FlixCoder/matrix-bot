//! The bot's database.

use bonsaidb::{
	core::schema::Schema,
	local::{
		config::{Builder, StorageConfiguration},
		AsyncDatabase,
	},
};
use bonsaimq::MessageQueueSchema;
use color_eyre::Result;

use crate::settings::Settings;

/// Open all databases as specified from the config.
pub async fn open_databases(config: &Settings) -> Result<Databases> {
	let state =
		AsyncDatabase::open::<BotSchema>(StorageConfiguration::new(&config.store.database)).await?;
	let jobs = AsyncDatabase::open::<MessageQueueSchema>(StorageConfiguration::new(
		&config.store.job_runner_db,
	))
	.await?;
	Ok(Databases { state, jobs })
}

/// A container for all databases used.
#[derive(Debug, Clone)]
pub struct Databases {
	/// The matrix bot's state database.
	pub state: AsyncDatabase,
	/// Database for the job/message queue.
	pub jobs: AsyncDatabase,
}

/// The bot's database schema for saving state.
#[derive(Debug, Schema)]
#[schema(name = "matrix_bot", collections = [])]
pub struct BotSchema;
