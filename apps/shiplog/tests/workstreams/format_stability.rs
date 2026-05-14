//! Format stability tests for shiplog-workstreams.
//!
//! Complements `snapshot.rs` with YAML round-trip snapshots through `write_workstreams`,
//! the curated/suggested file contract, and edge-case values to guard against
//! accidental serialization changes in the workstreams YAML format.

use chrono::{TimeZone, Utc};
use shiplog::ids::{EventId, WorkstreamId};
use shiplog::schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog::workstreams::write_workstreams;

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

// ============================================================================
// Full workstreams.yaml through write_workstreams (the actual YAML writer)
// ============================================================================

#[test]
fn format_stability_workstreams_yaml_full() {
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
                receipts: vec![
                    EventId::from_parts(["github", "pr", "acme/widgets", "10"]),
                    EventId::from_parts(["github", "pr", "acme/widgets", "11"]),
                ],
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

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_full", text);
}

// ============================================================================
// workstreams.suggested.yaml through WorkstreamManager
// ============================================================================

#[test]
fn format_stability_workstreams_suggested_yaml() {
    use shiplog::workstreams::WorkstreamManager;

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["repo", "acme/api"]),
            title: "API Service".into(),
            summary: Some("REST API development".into()),
            tags: vec!["api".into(), "backend".into()],
            stats: WorkstreamStats {
                pull_requests: 5,
                reviews: 3,
                manual_events: 1,
            },
            events: vec![
                EventId::from_parts(["github", "pr", "acme/api", "1"]),
                EventId::from_parts(["github", "pr", "acme/api", "2"]),
            ],
            receipts: vec![EventId::from_parts(["github", "pr", "acme/api", "1"])],
        }],
    };

    let dir = tempfile::tempdir().unwrap();
    WorkstreamManager::write_suggested(dir.path(), &ws).unwrap();

    let path = WorkstreamManager::suggested_path(dir.path());
    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_suggested_yaml", text);
}

// ============================================================================
// Empty workstreams file
// ============================================================================

#[test]
fn format_stability_workstreams_yaml_empty() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![],
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_empty", text);
}

// ============================================================================
// Minimal workstream (only required fields, optionals absent)
// ============================================================================

#[test]
fn format_stability_workstreams_yaml_minimal() {
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

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_minimal", text);
}

// ============================================================================
// Edge cases: special characters, Unicode, large values
// ============================================================================

#[test]
fn format_stability_workstreams_yaml_edge_cases() {
    let ws = WorkstreamsFile {
        version: u32::MAX,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["edge"]),
            title: "Title with \"quotes\" & <angle brackets> 🚀".into(),
            summary: Some("Multi-line\nsummary with\ttabs".into()),
            tags: vec!["".into(), "tag with spaces".into(), "日本語タグ".into()],
            stats: WorkstreamStats {
                pull_requests: usize::MAX,
                reviews: 0,
                manual_events: usize::MAX,
            },
            events: vec![EventId::from_parts(["special", "chars", "a&b<c>d"])],
            receipts: vec![],
        }],
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    write_workstreams(&path, &ws).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_edge_cases", text);
}

// ============================================================================
// YAML round-trip: write then read back preserves structure
// ============================================================================

#[test]
fn format_stability_workstreams_yaml_roundtrip() {
    let original = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["ws", "auth"]),
                title: "Authentication".into(),
                summary: Some("OAuth2 integration".into()),
                tags: vec!["security".into()],
                stats: WorkstreamStats {
                    pull_requests: 5,
                    reviews: 3,
                    manual_events: 1,
                },
                events: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
                receipts: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
            },
            Workstream {
                id: WorkstreamId::from_parts(["ws", "perf"]),
                title: "Performance".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workstreams.yaml");
    write_workstreams(&path, &original).unwrap();

    // Snapshot the written YAML (the format under test)
    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("workstreams_yaml_roundtrip", text);

    // Verify round-trip fidelity
    let loaded: WorkstreamsFile = serde_yaml::from_str(&text).unwrap();
    assert_eq!(loaded.version, original.version);
    assert_eq!(loaded.workstreams.len(), original.workstreams.len());
    assert_eq!(loaded.workstreams[0].title, "Authentication");
    assert_eq!(loaded.workstreams[1].title, "Performance");
}
