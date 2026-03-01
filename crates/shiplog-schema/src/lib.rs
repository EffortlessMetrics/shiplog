#![warn(missing_docs)]
//! Canonical event model and data types for the shiplog pipeline.
//!
//! Defines event envelopes, event payloads (pull requests, reviews, manual entries),
//! coverage manifests, workstream definitions, and bundle metadata.
//! All other crates depend on these types.
//!
//! # Examples
//!
//! Build an [`event::EventEnvelope`] from scratch:
//!
//! ```
//! use shiplog_schema::event::*;
//! use shiplog_schema::coverage::TimeWindow;
//! use shiplog_ids::EventId;
//! use chrono::{TimeZone, Utc};
//!
//! let ts = Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap();
//! let envelope = EventEnvelope {
//!     id: EventId::from_parts(["github", "pr", "acme/widgets", "42"]),
//!     kind: EventKind::PullRequest,
//!     occurred_at: ts,
//!     actor: Actor { login: "octocat".into(), id: Some(1) },
//!     repo: RepoRef {
//!         full_name: "acme/widgets".into(),
//!         html_url: Some("https://github.com/acme/widgets".into()),
//!         visibility: RepoVisibility::Public,
//!     },
//!     payload: EventPayload::PullRequest(PullRequestEvent {
//!         number: 42,
//!         title: "Add feature".into(),
//!         state: PullRequestState::Merged,
//!         created_at: ts,
//!         merged_at: Some(ts),
//!         additions: Some(100),
//!         deletions: Some(20),
//!         changed_files: Some(5),
//!         touched_paths_hint: vec!["src/lib.rs".into()],
//!         window: None,
//!     }),
//!     tags: vec!["feature".into()],
//!     links: vec![],
//!     source: SourceRef {
//!         system: SourceSystem::Github,
//!         url: None,
//!         opaque_id: None,
//!     },
//! };
//!
//! // Round-trips through JSON:
//! let json = serde_json::to_string(&envelope).unwrap();
//! let back: EventEnvelope = serde_json::from_str(&json).unwrap();
//! assert_eq!(envelope, back);
//! ```

/// Bundle metadata types for checksums and archive manifests.
pub mod bundle;
/// Coverage manifest types for tracking query completeness.
pub mod coverage;
/// Core event model: envelopes, payloads, actors, and source provenance.
pub mod event;
/// Workstream definitions and the top-level workstreams file format.
pub mod workstream;
