//! Intervalled execution of periodic tasks.

mod github;
mod rss;

use std::{sync::Arc, time::Duration};

use color_eyre::Result;
use matrix_sdk::Client;
use tokio::time::{interval, MissedTickBehavior};

use crate::{database::Databases, settings::Settings};

/// Run the intervals, logging and restarting on error.
pub async fn run(config: Arc<Settings>, databases: Databases, client: Client) {
	let mut state = State { github: github::IntervalState::default() };

	while let Err(err) = intervals(&config, &databases, &client, &mut state).await {
		tracing::error!("Error in intervals: {err}");
	}
}

/// State across interval executions, e.g. for caching.
struct State {
	/// Github interval state.
	github: github::IntervalState,
}

/// Run the actual intervals, returning on error.
async fn intervals(
	config: &Settings,
	databases: &Databases,
	client: &Client,
	state: &mut State,
) -> Result<()> {
	let mut rss_interval = interval(Duration::from_secs(config.intervals.rss));
	rss_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
	let mut github_interval = interval(Duration::from_secs(config.intervals.github));
	github_interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

	loop {
		tokio::select! {
			_ = rss_interval.tick() => rss::interval(databases, client).await?,
			_ = github_interval.tick() => github::interval(databases, client, &mut state.github).await?,
		};
	}
}
