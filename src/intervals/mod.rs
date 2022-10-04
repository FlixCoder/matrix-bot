//! Intervalled execution of periodic tasks.

mod rss;

use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use matrix_sdk::Client;
use tokio::time::{interval, MissedTickBehavior};

use crate::{database::Databases, settings::Settings};

/// Run the intervals, logging and restarting on error.
pub async fn run(config: Arc<Settings>, databases: Databases, client: Client) {
	while let Err(err) = intervals(&config, &databases, &client).await {
		tracing::error!("Error in intervals: {err}");
	}
}

/// Run the actual intervals, returning on error.
pub async fn intervals(config: &Settings, databases: &Databases, client: &Client) -> Result<()> {
	let mut rss_interval = interval(Duration::from_secs(config.intervals.rss));
	rss_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

	loop {
		tokio::select! {
			_ = rss_interval.tick() => rss::interval(databases, client).await?,
		};
	}
}
