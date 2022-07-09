//! Configuration module

use std::{path::PathBuf, str::FromStr};

use config::{ConfigError, Environment, File};
use serde::{de::Error, Deserialize, Deserializer};
use tracing::Level;

/// This app's configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
	/// Logging level
	#[serde(deserialize_with = "deserialize_log_level")]
	pub log_level: Level,
	/// Matrix login information
	pub login: LoginSettings,
	/// Persons who have access to the bot.
	pub access: AccessSettings,
}

impl Settings {
	/// Read configuration from `config.yaml` by default. Calls `read_from`.
	#[inline]
	pub fn read() -> Result<Self, ConfigError> {
		Self::read_from("config.yaml")
	}

	/// Read configuration from specified file and merge in environment variable
	/// configuration.
	pub fn read_from(cfg_path: &str) -> Result<Self, ConfigError> {
		let config = ::config::Config::builder()
			//.set_default("key", "value")?;
			.add_source(File::with_name(cfg_path).required(false))
			.add_source(Environment::with_prefix("APP").separator("__"))
			.build()?
			.try_deserialize()?;
		Ok(config)
	}
}

/// Login settings
#[derive(Debug, Clone, Deserialize)]
pub struct LoginSettings {
	/// Homeserver.
	pub home_server: String,
	/// Username.
	pub user: String,
	/// Password.
	pub password: String,
	/// Location of state-store
	pub state_store: PathBuf,
}

// TODO: Use MXIDs.
/// Access control settings
#[derive(Debug, Clone, Deserialize)]
pub struct AccessSettings {
	/// Admins (full access)
	pub admins: Vec<String>,
	/// Moderators (execute commands only)
	pub mods: Vec<String>,
}

/// Deserializes `String` into `tracing::Level`
pub fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
	D: Deserializer<'de>,
{
	let string = String::deserialize(deserializer)?;
	let level: Level =
		tracing::Level::from_str(&string).map_err(|error| D::Error::custom(error.to_string()))?;

	Ok(level)
}
