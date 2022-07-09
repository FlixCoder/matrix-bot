//! Crate library.

mod events;
pub mod settings;

use std::sync::{Arc, Barrier};

use color_eyre::Result;
use matrix_sdk::{config::SyncSettings, store::StateStore, Client};
use settings::Settings;

/// Log into matrix account.
async fn login(config: &Settings) -> Result<Client> {
	tracing::debug!("Opening state store..");
	let state_store = StateStore::open_with_path(&config.login.state_store)?;

	tracing::debug!("Logging in..");
	let client = Client::builder()
		.homeserver_url(&config.login.home_server)
		.state_store(state_store)
		.build()
		.await?;
	client
		.login_username(&config.login.user, &config.login.password)
		.initial_device_display_name("Matrix-Bot")
		.send()
		.await?;

	tracing::debug!("Initial sync..");
	client.sync_once(SyncSettings::default()).await?;

	tracing::info!("Logged in as {:?}", client.user_id());
	Ok(client)
}

/// Run the bot.
pub async fn run(config: &Settings) -> Result<()> {
	let stop_barrier = Arc::new(Barrier::new(2));
	let stopper = stop_barrier.clone();
	ctrlc::set_handler(move || {
		stopper.wait();
	})?;
	let client = login(config).await?;

	client.register_event_handler(events::on_room_message).await;

	tracing::info!("Running continuous sync..");
	let sync_settings = client
		.sync_token()
		.await
		.map(|sync_token| SyncSettings::default().token(sync_token))
		.unwrap_or_default();
	let sync_handle = tokio::spawn(async move {
		client.sync(sync_settings).await;
	});
	tokio::task::block_in_place(move || stop_barrier.wait());

	tracing::info!("Stopping the client..");
	sync_handle.abort();

	Ok(())
}
