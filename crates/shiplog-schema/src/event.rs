use crate::coverage::TimeWindow;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::EventId;

/// Where a record came from.
///
/// This is part of the trust story: a packet is only as good as its provenance.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceSystem {
    Github,
    JsonImport,
    LocalGit,
    Manual,
    Unknown,
    /// Extension point for third-party source systems.
    Other(String),
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
    Manual,
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
    Manual(ManualEvent),
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

/// Types of manual events for non-GitHub work.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ManualEventType {
    /// General note or achievement
    Note,
    /// Incident response or on-call
    Incident,
    /// Design doc or architecture work
    Design,
    /// Mentoring or teaching
    Mentoring,
    /// Feature or product launch
    Launch,
    /// Migration or infrastructure work
    Migration,
    /// Code review (non-GitHub)
    Review,
    /// Other uncategorized work
    Other,
}

/// Manual event for work that doesn't have GitHub artifacts.
///
/// This allows the packet to include:
/// - Incidents handled
/// - Migrations planned
/// - Mentoring
/// - Cross-team design
/// - Unmerged prototypes that still mattered
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ManualEvent {
    /// Type of manual event
    pub event_type: ManualEventType,
    /// Title/summary of the work
    pub title: String,
    /// Detailed description
    pub description: Option<String>,
    /// Start date (for multi-day work)
    pub started_at: Option<NaiveDate>,
    /// End/completion date
    pub ended_at: Option<NaiveDate>,
    /// Impact or outcome statement
    pub impact: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_system_round_trip_known_variants() {
        for variant in [
            SourceSystem::Github,
            SourceSystem::JsonImport,
            SourceSystem::LocalGit,
            SourceSystem::Manual,
            SourceSystem::Unknown,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: SourceSystem = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn source_system_other_round_trip() {
        let variant = SourceSystem::Other("gitlab".into());
        let json = serde_json::to_string(&variant).unwrap();
        let back: SourceSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
        assert!(json.contains("gitlab"));
    }
}

/// File format for manual_events.yaml
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ManualEventsFile {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub events: Vec<ManualEventEntry>,
}

/// Individual manual event entry with metadata
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ManualEventEntry {
    /// Unique identifier for this entry
    pub id: String,
    /// Event type
    #[serde(rename = "type")]
    pub event_type: ManualEventType,
    /// Date or date range
    pub date: ManualDate,
    /// Title of the work
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Workstream association
    pub workstream: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Receipts/links to evidence
    pub receipts: Vec<Link>,
    /// Impact statement
    pub impact: Option<String>,
}

/// Date specification for manual events
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ManualDate {
    Single(NaiveDate),
    Range { start: NaiveDate, end: NaiveDate },
}
