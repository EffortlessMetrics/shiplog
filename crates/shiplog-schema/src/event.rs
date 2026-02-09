use crate::coverage::TimeWindow;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::EventId;

/// Where a record came from.
///
/// This is part of the trust story: a packet is only as good as its provenance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceSystem {
    Github,
    JsonImport,
    LocalGit,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    pub system: SourceSystem,
    /// A stable URL when available. May be stripped during redaction.
    pub url: Option<String>,
    /// Provider specific opaque id (GitHub node_id, etc.).
    pub opaque_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    pub login: String,
    pub id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RepoVisibility {
    Public,
    Private,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoRef {
    /// "owner/name" when known.
    pub full_name: String,
    /// HTML URL (not API URL) when known.
    pub html_url: Option<String>,
    pub visibility: RepoVisibility,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum EventKind {
    PullRequest,
    Review,
}

/// The canonical event record.
///
/// This is the data spine. Everything else should be derived from it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EventEnvelope {
    pub id: EventId,
    pub kind: EventKind,
    pub occurred_at: DateTime<Utc>,
    pub actor: Actor,
    pub repo: RepoRef,
    pub payload: EventPayload,
    pub tags: Vec<String>,
    pub links: Vec<Link>,
    pub source: SourceRef,
}

/// Payload is tagged for forward-compatible evolution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    PullRequest(PullRequestEvent),
    Review(ReviewEvent),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
    Unknown,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PullRequestEvent {
    pub number: u64,
    pub title: String,
    pub state: PullRequestState,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    /// Minimal risk proxy. It's not "quality". It's blast radius.
    pub touched_paths_hint: Vec<String>,
    pub window: Option<TimeWindow>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ReviewEvent {
    pub pull_number: u64,
    pub pull_title: String,
    pub submitted_at: DateTime<Utc>,
    pub state: String,
    pub window: Option<TimeWindow>,
}
