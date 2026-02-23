use crate::coverage::TimeWindow;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::EventId;
use std::fmt;

/// Where a record came from.
///
/// This is part of the trust story: a packet is only as good as its provenance.
#[derive(Clone, Debug, PartialEq, Eq)]
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

impl SourceSystem {
    /// Canonical lowercase string for this variant.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Github => "github",
            Self::JsonImport => "json_import",
            Self::LocalGit => "local_git",
            Self::Manual => "manual",
            Self::Unknown => "unknown",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse from a string, case-insensitively matching known variants.
    /// Unrecognised strings become `Other(s)`.
    pub fn from_str_lossy(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "github" => Self::Github,
            "json_import" | "jsonimport" => Self::JsonImport,
            "local_git" | "localgit" => Self::LocalGit,
            "manual" => Self::Manual,
            "unknown" => Self::Unknown,
            _ => Self::Other(s.to_string()),
        }
    }
}

impl fmt::Display for SourceSystem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for SourceSystem {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for SourceSystem {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct SourceSystemVisitor;

        impl<'de> serde::de::Visitor<'de> for SourceSystemVisitor {
            type Value = SourceSystem;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a source system string or object")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<SourceSystem, E> {
                Ok(SourceSystem::from_str_lossy(v))
            }

            fn visit_map<A: serde::de::MapAccess<'de>>(
                self,
                mut map: A,
            ) -> Result<SourceSystem, A::Error> {
                let key: String = map
                    .next_key()?
                    .ok_or_else(|| serde::de::Error::custom("expected a single-key map"))?;

                let result = match key.to_ascii_lowercase().as_str() {
                    "github" | "jsonimport" | "json_import" | "localgit" | "local_git"
                    | "manual" | "unknown" => {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                        SourceSystem::from_str_lossy(&key)
                    }
                    "other" => {
                        let value: String = map.next_value()?;
                        SourceSystem::from_str_lossy(&value)
                    }
                    _ => {
                        let _: serde::de::IgnoredAny = map.next_value()?;
                        SourceSystem::Other(key)
                    }
                };

                if map.next_key::<String>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "expected a single-key map for SourceSystem",
                    ));
                }

                Ok(result)
            }
        }

        deserializer.deserialize_any(SourceSystemVisitor)
    }
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
        let cases = [
            (SourceSystem::Github, r#""github""#),
            (SourceSystem::JsonImport, r#""json_import""#),
            (SourceSystem::LocalGit, r#""local_git""#),
            (SourceSystem::Manual, r#""manual""#),
            (SourceSystem::Unknown, r#""unknown""#),
        ];
        for (variant, expected_json) in cases {
            let json = serde_json::to_string(&variant).unwrap();
            assert_eq!(json, expected_json, "serialize {:?}", variant);
            let back: SourceSystem = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back, "round-trip {:?}", variant);
        }
    }

    #[test]
    fn source_system_other_round_trip() {
        let variant = SourceSystem::Other("gitlab".into());
        let json = serde_json::to_string(&variant).unwrap();
        assert_eq!(json, r#""gitlab""#);
        let back: SourceSystem = serde_json::from_str(&json).unwrap();
        assert_eq!(variant, back);
    }

    #[test]
    fn source_system_other_does_not_collide_with_known() {
        let back: SourceSystem = serde_json::from_str(r#""github""#).unwrap();
        assert_eq!(back, SourceSystem::Github);
    }

    #[test]
    fn source_system_backward_compat_pascal_case() {
        // Old serialisation used PascalCase; must still deserialise.
        let cases = [
            (r#""Github""#, SourceSystem::Github),
            (r#""JsonImport""#, SourceSystem::JsonImport),
            (r#""LocalGit""#, SourceSystem::LocalGit),
            (r#""Manual""#, SourceSystem::Manual),
            (r#""Unknown""#, SourceSystem::Unknown),
        ];
        for (json, expected) in cases {
            let back: SourceSystem = serde_json::from_str(json).unwrap();
            assert_eq!(back, expected, "backward compat for {json}");
        }
    }

    #[test]
    fn source_system_backward_compat_object_form_unit_variants() {
        let cases = [
            (r#"{"Github":null}"#, SourceSystem::Github),
            (r#"{"JsonImport":null}"#, SourceSystem::JsonImport),
            (r#"{"LocalGit":null}"#, SourceSystem::LocalGit),
            (r#"{"Manual":null}"#, SourceSystem::Manual),
            (r#"{"Unknown":null}"#, SourceSystem::Unknown),
        ];
        for (json, expected) in cases {
            let back: SourceSystem = serde_json::from_str(json).unwrap();
            assert_eq!(back, expected, "backward compat object form for {json}");
        }
    }

    #[test]
    fn source_system_backward_compat_object_form_other() {
        let back: SourceSystem = serde_json::from_str(r#"{"Other":"gitlab"}"#).unwrap();
        assert_eq!(back, SourceSystem::Other("gitlab".into()));
    }

    #[test]
    fn source_system_backward_compat_object_form_other_known_name() {
        // {"Other":"github"} should normalise to Github, not Other("github")
        let back: SourceSystem = serde_json::from_str(r#"{"Other":"github"}"#).unwrap();
        assert_eq!(back, SourceSystem::Github);
    }

    #[test]
    fn source_system_object_form_rejects_multi_key_map() {
        let result = serde_json::from_str::<SourceSystem>(r#"{"Github":null,"Other":"x"}"#);
        assert!(result.is_err(), "multi-key map should be rejected");
    }

    #[test]
    fn source_system_display_matches_serde() {
        for variant in [
            SourceSystem::Github,
            SourceSystem::JsonImport,
            SourceSystem::LocalGit,
            SourceSystem::Manual,
            SourceSystem::Unknown,
            SourceSystem::Other("gitlab".into()),
        ] {
            let display = format!("{variant}");
            let serialized: String =
                serde_json::from_str(&serde_json::to_string(&variant).unwrap()).unwrap();
            assert_eq!(display, serialized, "Display vs serde for {:?}", variant);
        }
    }

    #[test]
    fn source_system_rejects_wrong_type_with_expecting_message() {
        let result = serde_json::from_str::<SourceSystem>("42");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("a source system string or object"),
            "expected 'expecting' message in error, got: {err}"
        );
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
