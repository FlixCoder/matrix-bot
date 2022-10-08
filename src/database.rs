//! The bot's database.

use std::collections::BTreeMap;

use bonsaidb::{
	core::{
		connection::AsyncConnection,
		document::{CollectionDocument, DocumentId, Emit},
		schema::{
			Collection, CollectionViewSchema, Schema, SerializedCollection, View, ViewMapResult,
		},
	},
	local::{
		config::{Builder, StorageConfiguration},
		AsyncDatabase,
	},
};
use bonsaimq::MessageQueueSchema;
use color_eyre::Result;
use matrix_sdk::ruma::{OwnedRoomId, RoomId};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use url::Url;

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
#[schema(name = "matrix_bot", collections = [RssSubscription, GithubSubscription])]
pub struct BotSchema;

/// Document entry for one RSS subscription.
#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "rss_subscriptions", views = [RssSubByRoom])]
pub struct RssSubscription {
	/// Matrix room ID for the subscription.
	pub room: OwnedRoomId,
	/// Feed URL.
	pub url: Url,
	/// Latest update posted into the room.
	pub latest_update: OffsetDateTime,
}

impl RssSubscription {
	/// Create a new RSS subscription for the current time.
	pub fn new(room: OwnedRoomId, url: Url) -> Self {
		Self { room, url, latest_update: OffsetDateTime::now_utc() }
	}

	/// Get RSS subscriptions for a specific room.
	pub async fn for_room(
		room: &RoomId,
		db: &AsyncDatabase,
	) -> Result<BTreeMap<DocumentId, CollectionDocument<Self>>, bonsaidb::core::Error> {
		let subscriptions = db
			.view::<RssSubByRoom>()
			.with_key(room.to_string())
			.query_with_collection_docs()
			.await?
			.documents;
		Ok(subscriptions)
	}

	/// Find a RSS subscription by room ID and URL.
	pub async fn find(
		room: &RoomId,
		url: &Url,
		db: &AsyncDatabase,
	) -> Result<Option<CollectionDocument<Self>>, bonsaidb::core::Error> {
		Ok(Self::for_room(room, db).await?.into_values().find(|doc| doc.contents.url == *url))
	}

	/// Insert the given RSS subscription into the database.
	pub async fn insert(self, db: &AsyncDatabase) -> Result<(), bonsaidb::core::Error> {
		if let Some(mut current) = Self::find(&self.room, &self.url, db).await? {
			current.contents.latest_update = self.latest_update;
			current.update_async(db).await?;
		} else {
			self.push_into_async(db).await?;
		}
		Ok(())
	}
}

/// View on RSS subscriptions by room ID.
#[derive(Debug, Clone, View)]
#[view(collection = RssSubscription, name = "rss_subscriptions_by_room", key = String, value = ())]
pub struct RssSubByRoom;

impl CollectionViewSchema for RssSubByRoom {
	type View = Self;

	fn map(&self, document: CollectionDocument<RssSubscription>) -> ViewMapResult<Self::View> {
		document.header.emit_key_and_value(document.contents.room.to_string(), ())
	}

	fn unique(&self) -> bool {
		false
	}

	fn version(&self) -> u64 {
		0
	}
}

/// Document entry for one Github notifications subscription.
#[derive(Debug, Clone, Serialize, Deserialize, Collection)]
#[collection(name = "github_subscriptions", views = [GithubSubByRoom])]
pub struct GithubSubscription {
	/// Matrix room ID for the subscription.
	pub room: OwnedRoomId,
	/// User name.
	pub user: String,
	/// Access token.
	pub token: String,
	/// Latest update posted into the room.
	pub latest_update: OffsetDateTime,
}

impl GithubSubscription {
	/// Create a new Github subscription for the current time.
	pub fn new(room: OwnedRoomId, user: String, token: String) -> Self {
		Self { room, user, token, latest_update: OffsetDateTime::now_utc() }
	}

	/// Get Github subscriptions for a specific room.
	pub async fn for_room(
		room: &RoomId,
		db: &AsyncDatabase,
	) -> Result<BTreeMap<DocumentId, CollectionDocument<Self>>, bonsaidb::core::Error> {
		let subscriptions = db
			.view::<GithubSubByRoom>()
			.with_key(room.to_string())
			.query_with_collection_docs()
			.await?
			.documents;
		Ok(subscriptions)
	}

	/// Find a Github subscription by room ID and user.
	pub async fn find(
		room: &RoomId,
		user: &str,
		db: &AsyncDatabase,
	) -> Result<Option<CollectionDocument<Self>>, bonsaidb::core::Error> {
		Ok(Self::for_room(room, db)
			.await?
			.into_values()
			.find(|doc| doc.contents.user.as_str() == user))
	}

	/// Insert the given Github subscription into the database.
	pub async fn insert(self, db: &AsyncDatabase) -> Result<(), bonsaidb::core::Error> {
		if let Some(mut current) = Self::find(&self.room, &self.user, db).await? {
			current.contents.token = self.token;
			current.contents.latest_update = self.latest_update;
			current.update_async(db).await?;
		} else {
			self.push_into_async(db).await?;
		}
		Ok(())
	}
}

/// View on Github subscriptions by room ID.
#[derive(Debug, Clone, View)]
#[view(collection = GithubSubscription, name = "github_subscriptions_by_room", key = String, value = ())]
pub struct GithubSubByRoom;

impl CollectionViewSchema for GithubSubByRoom {
	type View = Self;

	fn map(&self, document: CollectionDocument<GithubSubscription>) -> ViewMapResult<Self::View> {
		document.header.emit_key_and_value(document.contents.room.to_string(), ())
	}

	fn unique(&self) -> bool {
		false
	}

	fn version(&self) -> u64 {
		0
	}
}
