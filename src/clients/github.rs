//! API client functionality for Github.

use std::time::Duration;

use color_eyre::Result;
use reqwest::{
	header::{self, HeaderMap},
	Client, StatusCode, Url,
};
use serde::{Deserialize, Serialize};
use time::{
	format_description::well_known::{Rfc2822, Rfc3339},
	OffsetDateTime,
};

/// Base URL of the Github API.
const API_URL: &str = "https://api.github.com/";
/// User agent to use for Github requests.
const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

/// API client for Github notifications.
#[derive(Debug)]
pub struct Github {
	/// Request client.
	client: Client,
	/// Base API url.
	base_url: Url,
	/// Username
	user: String,
	/// Access token to access the API.
	token: String,
	/// Next allowed request time.
	allowed_request_time: OffsetDateTime,
}

impl Github {
	/// Create new Github client to the default API URL.
	pub fn new(username: String, token: String) -> Result<Self> {
		let mut default_headers = HeaderMap::new();
		default_headers.insert(header::USER_AGENT, USER_AGENT.parse()?);
		let client = Client::builder().default_headers(default_headers).build()?;

		Ok(Self {
			client,
			base_url: API_URL.parse()?,
			user: username,
			token,
			allowed_request_time: OffsetDateTime::UNIX_EPOCH,
		})
	}

	/// Set the token to the new value.
	pub fn set_token(&mut self, token: String) -> &mut Self {
		self.token = token;
		self
	}

	/// Test a token for validity.
	pub async fn test_token(&self) -> Result<()> {
		let _resp = self
			.client
			.head(self.base_url.join("notifications")?)
			.basic_auth(&self.user, Some(&self.token))
			.header(header::IF_MODIFIED_SINCE, OffsetDateTime::now_utc().format(&Rfc2822)?)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	/// Get whether the next request is already allowed.
	pub fn next_request_allowed(&self) -> bool {
		self.allowed_request_time < OffsetDateTime::now_utc()
	}

	/// List notifications since a specific point in time.
	pub async fn notifications(&mut self, since: OffsetDateTime) -> Result<Vec<Notification>> {
		let since_rfc2822 = since.format(&Rfc2822)?;
		let since_rfc3339 = since.format(&Rfc3339)?;
		let query = [("all", "false"), ("per_page", "50"), ("since", &since_rfc3339)];
		let response = self
			.client
			.get(self.base_url.join("notifications")?)
			.basic_auth(&self.user, Some(&self.token))
			.header(header::ACCEPT, "application/vnd.github+json")
			.header(header::IF_MODIFIED_SINCE, since_rfc2822)
			.query(&query)
			.send()
			.await?
			.error_for_status()?;

		if let Some(next_request) = response.headers().get("X-Poll-Interval") {
			let wait_duration = Duration::from_secs(next_request.to_str()?.parse()?);
			self.allowed_request_time = OffsetDateTime::now_utc() + wait_duration;
		}

		if response.status() == StatusCode::NOT_MODIFIED {
			return Ok(vec![]);
		}

		let entries: Vec<Notification> = response.json().await?;
		Ok(entries)
	}
}

/// API Response type for Github notifications.
#[derive(Debug, Serialize, Deserialize)]
pub struct Notification {
	/// ID.
	pub id: String,
	/// Last read at datetime.
	#[serde(with = "time::serde::iso8601::option")]
	pub last_read_at: Option<OffsetDateTime>,
	/// Notification reason.
	pub reason: NotificationReason,
	/// Raw repository information.
	pub repository: MinimalRepository,
	/// Subject.
	pub subject: Subject,
	/// Subscription URL.
	pub subscription_url: Url,
	/// Whether the notification is unread.
	pub unread: bool,
	/// Updated at datetime.
	#[serde(with = "time::serde::iso8601")]
	pub updated_at: OffsetDateTime,
	/// Notification URL.
	pub url: Url,
}

/// Reason for notification.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationReason {
	/// You were assigned to the issue.
	Assign,
	/// You created the thread.
	Author,
	/// You commented on the thread.
	Comment,
	/// A GitHub Actions workflow run that you triggered was completed.
	CiActivity,
	/// You accepted an invitation to contribute to the repository.
	Invitation,
	/// You subscribed to the thread (via an issue or pull request).
	Manual,
	/// You were specifically @mentioned in the content.
	Mention,
	/// You, or a team you're a member of, were requested to review a pull
	/// request.
	ReviewRequested,
	/// GitHub discovered a security vulnerability in your repository.
	SecurityAlert,
	/// You changed the thread state (for example, closing an issue or merging a
	/// pull request).
	StateChange,
	/// You're watching the repository.
	Subscribed,
	/// You were on a team that was mentioned.
	TeamMention,
}

/// Minimal Repository. TODO: This is incomplete!
#[derive(Debug, Serialize, Deserialize)]
pub struct MinimalRepository {
	/// Repository description.
	#[serde(default)]
	pub description: Option<String>,
	/// Whether this is a fork.
	pub fork: bool,
	/// Full repository name ("owner/name").
	pub full_name: String,
	/// HTML URL of the repository.
	pub html_url: Url,
	/// ID.
	pub id: u64,
	/// Node ID.
	pub node_id: String,
	/// Repository name.
	pub name: String,
	/// Whether this is a private repository.
	pub private: bool,
	/// URL.
	pub url: Url,
}

/// Notification subject.
#[derive(Debug, Serialize, Deserialize)]
pub struct Subject {
	/// Last comment URL.
	pub latest_comment_url: Option<Url>,
	/// Title.
	pub title: String,
	/// Type.
	pub r#type: String,
	/// URL.
	pub url: Option<Url>,
}
