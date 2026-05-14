//! Snapshot tests for workstream file serialization format.
//!
//! Locks down the YAML and JSON output of WorkstreamsFile to catch accidental
//! serialization changes.

use chrono::{TimeZone, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

// ============================================================================
// Fully populated WorkstreamsFile
// ============================================================================

#[test]
fn workstreams_file_full_yaml() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["repo", "acme/widgets"]),
                title: "Widgets Core".into(),
                summary: Some("Core widget library improvements".into()),
                tags: vec!["core".into(), "library".into()],
                stats: WorkstreamStats {
                    pull_requests: 8,
                    reviews: 12,
                    manual_events: 2,
                },
                events: vec![
                    EventId::from_parts(["github", "pr", "acme/widgets", "10"]),
                    EventId::from_parts(["github", "pr", "acme/widgets", "11"]),
                    EventId::from_parts(["github", "review", "acme/widgets", "10", "approved"]),
                ],
                receipts: vec![EventId::from_parts(["github", "pr", "acme/widgets", "10"])],
            },
            Workstream {
                id: WorkstreamId::from_parts(["repo", "acme/infra"]),
                title: "Infrastructure".into(),
                summary: Some("CI/CD and deployment pipelines".into()),
                tags: vec!["infra".into(), "devops".into()],
                stats: WorkstreamStats {
                    pull_requests: 3,
                    reviews: 1,
                    manual_events: 0,
                },
                events: vec![EventId::from_parts(["github", "pr", "acme/infra", "5"])],
                receipts: vec![EventId::from_parts(["github", "pr", "acme/infra", "5"])],
            },
        ],
    };
    insta::assert_yaml_snapshot!(ws);
}

#[test]
fn workstreams_file_full_json() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["repo", "acme/widgets"]),
            title: "Widgets Core".into(),
            summary: Some("Core widget library improvements".into()),
            tags: vec!["core".into()],
            stats: WorkstreamStats {
                pull_requests: 5,
                reviews: 3,
                manual_events: 1,
            },
            events: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
            receipts: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
        }],
    };
    insta::assert_json_snapshot!(ws);
}

// ============================================================================
// Minimal / empty
// ============================================================================

#[test]
fn workstreams_file_empty_yaml() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![],
    };
    insta::assert_yaml_snapshot!(ws);
}

#[test]
fn workstreams_file_minimal_workstream() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["min"]),
            title: "x".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };
    insta::assert_json_snapshot!(ws);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn workstreams_file_edge_cases() {
    let ws = WorkstreamsFile {
        version: u32::MAX,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["edge"]),
            title: "Title with \"quotes\" & <angle brackets> 🚀".into(),
            summary: Some("".into()),
            tags: vec!["".into(), "tag with spaces".into(), "日本語タグ".into()],
            stats: WorkstreamStats {
                pull_requests: usize::MAX,
                reviews: 0,
                manual_events: usize::MAX,
            },
            events: vec![EventId::from_parts([""])],
            receipts: vec![],
        }],
    };
    insta::assert_json_snapshot!(ws);
}
