//! Crate library.

mod clients;
mod commands;
mod database;
mod events;
mod intervals;
mod jobs;
mod matrix;
pub mod settings;

use std::{
	sync::{Arc, Barrier},
	time::Duration,
};

use bonsaimq::JobRunner;
use color_eyre::Result;
use matrix_sdk::{
	config::{RequestConfig, SyncSettings},
	Client,
};

use crate::{
	database::{open_databases, Databases},
	jobs::JobRegistry,
	matrix::ClientExt,
	settings::Settings,
};

/// Log into matrix account.
async fn login(config: &Settings) -> Result<Client> {
	tracing::debug!("Opening state store..");
	let client = Client::builder()
		.request_config(
			RequestConfig::short_retry().timeout(Duration::from_secs(config.request_timeout)),
		)
		.homeserver_url(&config.login.home_server)
		.sled_store(&config.store.state_store, Some(config.store.passphrase.as_str()))?
		.build()
		.await?;

	tracing::debug!("Attempting to restore login..");
	if !client.restore_session().await? {
		tracing::debug!("No session data, attempting login instead..");
		client
			.login_username(&config.login.user, &config.login.password)
			.initial_device_display_name("Matrix-Bot")
			.send()
			.await?;
	}

	tracing::info!("Logged in as {:?}", client.user_id());
	client.save_session().await?;
	Ok(client)
}

/// Join rooms that we are invited to if the user is allowed to invite us.
async fn process_invites(config: &Settings, client: &Client) -> Result<()> {
	tracing::debug!("Checking room invites..");
	for room in client.invited_rooms() {
		let room_name = room.name().unwrap_or_else(|| room.room_id().to_string());
		if let Some(inviter) = room.invite_details().await?.inviter {
			let inviter = inviter.user_id().to_owned();
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
#[tracing::instrument(level = "debug", skip_all, err)]
async fn matrix_run(config: Arc<Settings>, databases: Databases, client: Client) -> Result<()> {
	tracing::debug!("Initial sync..");
	client.sync_once(SyncSettings::default()).await?;

	client.leave_empty_rooms().await?;
	process_invites(&config, &client).await?;

	client.add_event_handler_context(config);
	client.add_event_handler_context(databases);
	client.add_event_handler(events::on_invite_event);
	client.add_event_handler(events::on_room_membership_event);
	client.add_event_handler(events::on_room_message);

	tracing::info!("Running continuous sync..");
	let sync_settings = client
		.sync_token()
		.await
		.map(|sync_token| SyncSettings::default().token(sync_token))
		.unwrap_or_default();
	client.sync(sync_settings).await?;

	Ok(())
}

/// Run the bot.
pub async fn run(config: Arc<Settings>) -> Result<()> {
	let stop_barrier = Arc::new(Barrier::new(2));
	let stopper = stop_barrier.clone();
	ctrlc::set_handler(move || {
		stopper.wait();
	})?;

	let databases = open_databases(&config).await?;
	let client = login(&config).await?;

	let sync_handle = tokio::spawn(matrix_run(config.clone(), databases.clone(), client.clone()));
	let _job_runner_handle = JobRunner::new(databases.jobs.clone())
		.set_context(config.clone())
		.set_context(databases.clone())
		.set_context(client.clone())
		.run::<JobRegistry>();
	let intervals_handle = tokio::spawn(intervals::run(config, databases, client.clone()));

	let termination_waiter = tokio::task::spawn_blocking(move || stop_barrier.wait());
	tokio::select! {
		res = termination_waiter => { res?; },
		res = sync_handle => res??,
		res = intervals_handle => res?,
	};

	tracing::info!("Stopping the client..");
	client.save_session().await?;
	Ok(())
}
