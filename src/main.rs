//! Main executable.

use color_eyre::Result;
use matrix_bot::settings::Settings;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
	color_eyre::install()?;
	dotenv::dotenv()?;
	let config = Settings::read()?;

	let filter = EnvFilter::from_default_env()
		.add_directive(config.log_level.into())
		.add_directive("mio=info".parse()?)
		.add_directive("want=info".parse()?);
	let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
	tracing::subscriber::set_global_default(subscriber)?;

	Ok(())
}
