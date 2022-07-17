//! Crate library.

mod commands;
mod events;
mod jobs;
mod matrix;
mod matrix_fix;
pub mod settings;

use std::sync::{Arc, Barrier};

use bonsaidb::local::{
	config::{Builder, StorageConfiguration},
	AsyncDatabase,
};
use bonsaimq::{JobRunner, MessageQueueSchema};
use color_eyre::Result;
use matrix_sdk::{config::SyncSettings, store::StateStore, Client};
use settings::Settings;

use crate::{jobs::JobRegistry, matrix::get_unique_members};

/// Log into matrix account.
async fn login(config: &Settings) -> Result<Client> {
	tracing::debug!("Opening state store..");
	let state_store = StateStore::open_with_path(&config.store.state_store)?;

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

	tracing::info!("Logged in as {:?}", client.user_id());
	Ok(client)
}

/// Leave empty rooms.
async fn leave_empty_rooms(client: &Client) -> Result<()> {
	tracing::debug!("Leaving empty rooms..");
	for room in client.joined_rooms() {
		let members = room.active_members().await?;
		if get_unique_members(&members) <= 1 {
			tracing::info!("Leaving room {} ({})", room.display_name().await?, room.room_id());
			room.leave().await?;
		}
	}
	Ok(())
}

/// Join rooms that we are invited to if the user is allowed to invite us.
async fn process_invites(config: &Settings, client: &Client) -> Result<()> {
	tracing::debug!("Checking room invites..");
	for room in client.invited_rooms() {
		let room_name = room.name().unwrap_or_else(|| room.room_id().to_string());
		// TODO: Do this instead when <https://github.com/matrix-org/matrix-rust-sdk/issues/833> is resolved.
		//if let Some(inviter) = room.invite_details().await?.inviter {
		//let inviter = inviter.user_id().to_owned();
		if let Some(inviter) = matrix_fix::invite_get_sender(&room).await? {
			if config.access.admins.contains(&inviter) {
				tracing::info!("Joining room {room_name}");
				room.accept_invitation().await?;
			} else {
				tracing::info!("Rejecting invitation to {room_name} from {inviter}");
				room.reject_invitation().await?;
			}
		}
	}
	Ok(())
}

/// Run the matrix setup and sync event loop.
async fn matrix_run(config: Settings, db: AsyncDatabase, client: Client) -> Result<()> {
	tracing::debug!("Initial sync..");
	client.sync_once(SyncSettings::default()).await?;

	leave_empty_rooms(&client).await?;
	process_invites(&config, &client).await?;

	client.register_event_handler_context(config);
	client.register_event_handler_context(db);
	client.register_event_handler(events::on_invite_event).await;
	client.register_event_handler(events::on_room_membership_event).await;
	client.register_event_handler(events::on_room_message).await;

	tracing::info!("Running continuous sync..");
	let sync_settings = client
		.sync_token()
		.await
		.map(|sync_token| SyncSettings::default().token(sync_token))
		.unwrap_or_default();
	client.sync(sync_settings).await;

	Ok(())
}

/// Run the bot.
pub async fn run(config: Settings) -> Result<()> {
	let stop_barrier = Arc::new(Barrier::new(2));
	let stopper = stop_barrier.clone();
	ctrlc::set_handler(move || {
		stopper.wait();
	})?;

	let db = AsyncDatabase::open::<MessageQueueSchema>(StorageConfiguration::new(
		&config.store.job_runner_db,
	))
	.await?;
	let client = login(&config).await?;

	let sync_handle = tokio::spawn(matrix_run(config.clone(), db.clone(), client.clone()));
	let _job_runner_handle =
		JobRunner::new(db).set_context(Arc::new(config)).set_context(client).run::<JobRegistry>();

	tokio::task::block_in_place(move || stop_barrier.wait());

	tracing::info!("Stopping the client..");
	sync_handle.abort();

	Ok(())
}
