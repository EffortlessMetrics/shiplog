use crate::coverage::TimeWindow;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::EventId;
use std::fmt;

/// Where a record came from.
///
/// This is part of the trust story: a packet is only as good as its provenance.
///
/// # Examples
///
/// ```
/// use shiplog_schema::event::SourceSystem;
///
/// let gh = SourceSystem::Github;
/// assert_eq!(gh.as_str(), "github");
///
/// // Round-trips through serde:
/// let json = serde_json::to_string(&gh).unwrap();
/// assert_eq!(json, r#""github""#);
/// let back: SourceSystem = serde_json::from_str(&json).unwrap();
/// assert_eq!(back, gh);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceSystem {
    /// GitHub API.
    Github,
    /// JSONL file import.
    JsonImport,
    /// Local git repository.
    LocalGit,
    /// Manually-entered event.
    Manual,
    /// Source could not be determined.
    Unknown,
    /// Extension point for third-party source systems.
    Other(String),
}

impl SourceSystem {
    /// Canonical lowercase string for this variant.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::event::SourceSystem;
    ///
    /// assert_eq!(SourceSystem::Github.as_str(), "github");
    /// assert_eq!(SourceSystem::JsonImport.as_str(), "json_import");
    /// assert_eq!(SourceSystem::Other("jira".into()).as_str(), "jira");
    /// ```
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
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::event::SourceSystem;
    ///
    /// assert_eq!(SourceSystem::from_str_lossy("GitHub"), SourceSystem::Github);
    /// assert_eq!(SourceSystem::from_str_lossy("manual"), SourceSystem::Manual);
    /// assert_eq!(
    ///     SourceSystem::from_str_lossy("jira"),
    ///     SourceSystem::Other("jira".into()),
    /// );
    /// ```
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

/// Provenance reference for an event, linking it back to its source system.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceRef {
    /// The system that produced this event.
    pub system: SourceSystem,
    /// A stable URL when available. May be stripped during redaction.
    pub url: Option<String>,
    /// Provider specific opaque id (GitHub node_id, etc.).
    pub opaque_id: Option<String>,
}

/// The person or bot that triggered an event.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Actor {
    /// GitHub login handle (e.g. `"octocat"`).
    pub login: String,
    /// Numeric user ID when known.
    pub id: Option<u64>,
}

/// Visibility level of a repository.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum RepoVisibility {
    /// Publicly accessible repository.
    Public,
    /// Private repository.
    Private,
    /// Visibility could not be determined.
    Unknown,
}

impl fmt::Display for RepoVisibility {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => f.write_str("Public"),
            Self::Private => f.write_str("Private"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// A reference to a GitHub repository with display metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RepoRef {
    /// "owner/name" when known.
    pub full_name: String,
    /// HTML URL (not API URL) when known.
    pub html_url: Option<String>,
    /// Repository visibility level.
    pub visibility: RepoVisibility,
}

/// A labelled hyperlink attached to an event (e.g. PR URL, postmortem).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Link {
    /// Human-readable label (e.g. `"pr"`, `"postmortem"`).
    pub label: String,
    /// Fully-qualified URL.
    pub url: String,
}

/// Discriminant for the top-level event kind.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EventKind {
    /// A pull request authored by the user.
    PullRequest,
    /// A code review submitted by the user.
    Review,
    /// A manually-entered event (non-GitHub work).
    Manual,
}

impl fmt::Display for EventKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PullRequest => f.write_str("PullRequest"),
            Self::Review => f.write_str("Review"),
            Self::Manual => f.write_str("Manual"),
        }
    }
}

/// The canonical event record.
///
/// This is the data spine. Everything else should be derived from it.
///
/// # Examples
///
/// ```
/// use shiplog_schema::event::*;
/// use shiplog_ids::EventId;
/// use chrono::{TimeZone, Utc};
///
/// let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
/// let ev = EventEnvelope {
///     id: EventId::from_parts(["github", "pr", "acme/widgets", "1"]),
///     kind: EventKind::PullRequest,
///     occurred_at: ts,
///     actor: Actor { login: "octocat".into(), id: Some(1) },
///     repo: RepoRef {
///         full_name: "acme/widgets".into(),
///         html_url: None,
///         visibility: RepoVisibility::Public,
///     },
///     payload: EventPayload::PullRequest(PullRequestEvent {
///         number: 1,
///         title: "Fix bug".into(),
///         state: PullRequestState::Merged,
///         created_at: ts,
///         merged_at: Some(ts),
///         additions: Some(10),
///         deletions: Some(2),
///         changed_files: Some(1),
///         touched_paths_hint: vec![],
///         window: None,
///     }),
///     tags: vec![],
///     links: vec![],
///     source: SourceRef {
///         system: SourceSystem::Github,
///         url: None,
///         opaque_id: None,
///     },
/// };
/// assert_eq!(ev.kind, EventKind::PullRequest);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventEnvelope {
    /// Deterministic, content-derived event identifier.
    pub id: EventId,
    /// Top-level discriminant for the event type.
    pub kind: EventKind,
    /// When the event happened (UTC).
    pub occurred_at: DateTime<Utc>,
    /// Who triggered the event.
    pub actor: Actor,
    /// Repository this event belongs to.
    pub repo: RepoRef,
    /// Type-specific event data.
    pub payload: EventPayload,
    /// Freeform tags for categorisation.
    pub tags: Vec<String>,
    /// Related hyperlinks (PR URL, postmortem, etc.).
    pub links: Vec<Link>,
    /// Provenance metadata linking back to the source system.
    pub source: SourceRef,
}

/// Payload is tagged for forward-compatible evolution.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "data")]
pub enum EventPayload {
    /// Pull request authored by the user.
    PullRequest(PullRequestEvent),
    /// Code review submitted by the user.
    Review(ReviewEvent),
    /// Manually-entered non-GitHub event.
    Manual(ManualEvent),
}

/// Lifecycle state of a pull request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PullRequestState {
    /// Still open for review.
    Open,
    /// Closed without merging.
    Closed,
    /// Merged into the target branch.
    Merged,
    /// State could not be determined.
    Unknown,
}

impl fmt::Display for PullRequestState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => f.write_str("Open"),
            Self::Closed => f.write_str("Closed"),
            Self::Merged => f.write_str("Merged"),
            Self::Unknown => f.write_str("Unknown"),
        }
    }
}

/// A pull request event with diff-stat metadata.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestEvent {
    /// PR number within the repository.
    pub number: u64,
    /// PR title.
    pub title: String,
    /// Current lifecycle state.
    pub state: PullRequestState,
    /// When the PR was opened.
    pub created_at: DateTime<Utc>,
    /// When the PR was merged, if applicable.
    pub merged_at: Option<DateTime<Utc>>,
    /// Lines added.
    pub additions: Option<u64>,
    /// Lines removed.
    pub deletions: Option<u64>,
    /// Number of files changed.
    pub changed_files: Option<u64>,
    /// Minimal risk proxy. It's not "quality". It's blast radius.
    pub touched_paths_hint: Vec<String>,
    /// Coverage window this event was fetched within.
    pub window: Option<TimeWindow>,
}

/// A code review submitted on a pull request.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewEvent {
    /// Number of the PR that was reviewed.
    pub pull_number: u64,
    /// Title of the PR that was reviewed.
    pub pull_title: String,
    /// When the review was submitted.
    pub submitted_at: DateTime<Utc>,
    /// Review verdict (e.g. `"approved"`, `"changes_requested"`).
    pub state: String,
    /// Coverage window this event was fetched within.
    pub window: Option<TimeWindow>,
}

/// Types of manual events for non-GitHub work.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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

impl fmt::Display for ManualEventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Note => f.write_str("Note"),
            Self::Incident => f.write_str("Incident"),
            Self::Design => f.write_str("Design"),
            Self::Mentoring => f.write_str("Mentoring"),
            Self::Launch => f.write_str("Launch"),
            Self::Migration => f.write_str("Migration"),
            Self::Review => f.write_str("Review"),
            Self::Other => f.write_str("Other"),
        }
    }
}

/// Manual event for work that doesn't have GitHub artifacts.
///
/// This allows the packet to include:
/// - Incidents handled
/// - Migrations planned
/// - Mentoring
/// - Cross-team design
/// - Unmerged prototypes that still mattered
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

    #[test]
    fn source_ref_serde_roundtrip() {
        let sr = SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/repos/acme/widgets/pulls/1".into()),
            opaque_id: Some("PR_abc".into()),
        };
        let json = serde_json::to_string(&sr).unwrap();
        let back: SourceRef = serde_json::from_str(&json).unwrap();
        assert_eq!(sr, back);
    }

    #[test]
    fn source_ref_optional_fields_absent() {
        let sr = SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        };
        let json = serde_json::to_string(&sr).unwrap();
        let back: SourceRef = serde_json::from_str(&json).unwrap();
        assert_eq!(sr, back);
    }

    #[test]
    fn actor_serde_roundtrip() {
        let actor = Actor {
            login: "octocat".into(),
            id: Some(12345),
        };
        let json = serde_json::to_string(&actor).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(actor, back);
    }

    #[test]
    fn actor_optional_id() {
        let actor = Actor {
            login: "ghost".into(),
            id: None,
        };
        let json = serde_json::to_string(&actor).unwrap();
        let back: Actor = serde_json::from_str(&json).unwrap();
        assert_eq!(actor, back);
    }

    #[test]
    fn repo_ref_serde_roundtrip() {
        let rr = RepoRef {
            full_name: "acme/widgets".into(),
            html_url: Some("https://github.com/acme/widgets".into()),
            visibility: RepoVisibility::Public,
        };
        let json = serde_json::to_string(&rr).unwrap();
        let back: RepoRef = serde_json::from_str(&json).unwrap();
        assert_eq!(rr, back);
    }

    #[test]
    fn link_serde_roundtrip() {
        let link = Link {
            label: "pr".into(),
            url: "https://github.com/acme/widgets/pull/42".into(),
        };
        let json = serde_json::to_string(&link).unwrap();
        let back: Link = serde_json::from_str(&json).unwrap();
        assert_eq!(link, back);
    }

    #[test]
    fn event_envelope_pr_serde_roundtrip() {
        use chrono::{TimeZone, Utc};

        let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
        let event = EventEnvelope {
            id: EventId::from_parts(["github", "pr", "acme/widgets", "42"]),
            kind: EventKind::PullRequest,
            occurred_at: ts,
            actor: Actor {
                login: "octocat".into(),
                id: Some(1),
            },
            repo: RepoRef {
                full_name: "acme/widgets".into(),
                html_url: Some("https://github.com/acme/widgets".into()),
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 42,
                title: "Add feature".into(),
                state: PullRequestState::Merged,
                created_at: ts,
                merged_at: Some(ts),
                additions: Some(100),
                deletions: Some(20),
                changed_files: Some(5),
                touched_paths_hint: vec!["src/lib.rs".into()],
                window: None,
            }),
            tags: vec!["feature".into()],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn event_envelope_review_serde_roundtrip() {
        use chrono::{TimeZone, Utc};

        let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
        let event = EventEnvelope {
            id: EventId::from_parts(["github", "review", "acme/widgets", "42", "1"]),
            kind: EventKind::Review,
            occurred_at: ts,
            actor: Actor {
                login: "reviewer".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/widgets".into(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: 42,
                pull_title: "Add feature".into(),
                submitted_at: ts,
                state: "approved".into(),
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn event_envelope_manual_serde_roundtrip() {
        use chrono::{TimeZone, Utc};

        let ts = Utc.with_ymd_and_hms(2025, 3, 15, 10, 0, 0).unwrap();
        let event = EventEnvelope {
            id: EventId::from_parts(["manual", "incident-1"]),
            kind: EventKind::Manual,
            occurred_at: ts,
            actor: Actor {
                login: "oncall".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/widgets".into(),
                html_url: None,
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Incident,
                title: "P1 incident".into(),
                description: Some("Responded to outage".into()),
                started_at: Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
                ended_at: Some(NaiveDate::from_ymd_opt(2025, 3, 16).unwrap()),
                impact: Some("Reduced MTTR".into()),
            }),
            tags: vec!["incident".into()],
            links: vec![Link {
                label: "postmortem".into(),
                url: "https://wiki/incident-1".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: None,
                opaque_id: None,
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: EventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn manual_events_file_serde_roundtrip() {
        use chrono::{TimeZone, Utc};

        let ts = Utc.with_ymd_and_hms(2025, 6, 1, 0, 0, 0).unwrap();
        let file = ManualEventsFile {
            version: 1,
            generated_at: ts,
            events: vec![ManualEventEntry {
                id: "entry-1".into(),
                event_type: ManualEventType::Design,
                date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 5, 1).unwrap()),
                title: "Architecture review".into(),
                description: Some("Reviewed microservice boundaries".into()),
                workstream: Some("platform".into()),
                tags: vec!["architecture".into()],
                receipts: vec![Link {
                    label: "doc".into(),
                    url: "https://docs/arch".into(),
                }],
                impact: Some("Improved service isolation".into()),
            }],
        };
        let json = serde_json::to_string(&file).unwrap();
        let back: ManualEventsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, back);
    }

    #[test]
    fn manual_event_entry_type_field_renamed() {
        // Verify #[serde(rename = "type")] works correctly
        use chrono::NaiveDate;

        let entry = ManualEventEntry {
            id: "e1".into(),
            event_type: ManualEventType::Note,
            date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            title: "Test".into(),
            description: None,
            workstream: None,
            tags: vec![],
            receipts: vec![],
            impact: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        // The JSON should contain "type" not "event_type"
        assert!(
            json.contains(r#""type":"#),
            "expected 'type' key in JSON, got: {json}"
        );
        assert!(
            !json.contains(r#""event_type":"#),
            "should not contain 'event_type' key in JSON, got: {json}"
        );
        let back: ManualEventEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }

    #[test]
    fn manual_date_range_in_entry() {
        let entry = ManualEventEntry {
            id: "e2".into(),
            event_type: ManualEventType::Migration,
            date: ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 3, 31).unwrap(),
            },
            title: "DB migration".into(),
            description: None,
            workstream: None,
            tags: vec![],
            receipts: vec![],
            impact: None,
        };
        let json = serde_json::to_string(&entry).unwrap();
        let back: ManualEventEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(entry, back);
    }
}

/// File format for manual_events.yaml
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManualEventsFile {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Timestamp when this file was generated.
    pub generated_at: DateTime<Utc>,
    /// List of manual event entries.
    pub events: Vec<ManualEventEntry>,
}

/// Individual manual event entry with metadata
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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

/// Date specification for manual events — either a single day or a range.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ManualDate {
    /// A single calendar date.
    Single(NaiveDate),
    /// An inclusive date range.
    Range {
        /// Start of the range.
        start: NaiveDate,
        /// End of the range.
        end: NaiveDate,
    },
}
