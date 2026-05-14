//! Format stability tests for shiplog-render-json.
//!
//! Complements `snapshot.rs` with JSONL line-level snapshots for each event type
//! (PR, Review, Manual), full coverage manifest JSON snapshots, and edge-case
//! values to guard against accidental serialization changes.

use crate::artifact_json::{write_coverage_manifest, write_events_jsonl};
use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::*;

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

fn window() -> TimeWindow {
    TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }
}

fn pr_envelope() -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", "acme/widgets", "42"]),
        kind: EventKind::PullRequest,
        occurred_at: ts(),
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
            title: "Add feature X".into(),
            state: PullRequestState::Merged,
            created_at: ts(),
            merged_at: Some(ts()),
            additions: Some(100),
            deletions: Some(20),
            changed_files: Some(5),
            touched_paths_hint: vec!["src/lib.rs".into(), "tests/integration.rs".into()],
            window: Some(window()),
        }),
        tags: vec!["feature".into()],
        links: vec![Link {
            label: "pr".into(),
            url: "https://github.com/acme/widgets/pull/42".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/repos/acme/widgets/pulls/42".into()),
            opaque_id: Some("PR_abc123".into()),
        },
    }
}

fn review_envelope() -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "review", "acme/widgets", "42", "1"]),
        kind: EventKind::Review,
        occurred_at: ts(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(2),
        },
        repo: RepoRef {
            full_name: "acme/widgets".into(),
            html_url: Some("https://github.com/acme/widgets".into()),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "Add feature X".into(),
            submitted_at: ts(),
            state: "approved".into(),
            window: Some(window()),
        }),
        tags: vec!["review".into()],
        links: vec![Link {
            label: "pr".into(),
            url: "https://github.com/acme/widgets/pull/42".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: Some("PRR_def456".into()),
        },
    }
}

fn manual_envelope() -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["manual", "acme/widgets", "incident-1"]),
        kind: EventKind::Manual,
        occurred_at: ts(),
        actor: Actor {
            login: "oncall-eng".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/widgets".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "P1 incident response".into(),
            description: Some("Responded to production outage affecting payments".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 3, 16).unwrap()),
            impact: Some("Reduced MTTR by 50%".into()),
        }),
        tags: vec!["incident".into(), "oncall".into()],
        links: vec![Link {
            label: "postmortem".into(),
            url: "https://wiki.internal/incident-42".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

// ============================================================================
// JSONL: individual event type lines
// ============================================================================

#[test]
fn format_stability_jsonl_pr_event() {
    let events = vec![pr_envelope()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("jsonl_pr_event", text);
}

#[test]
fn format_stability_jsonl_review_event() {
    let events = vec![review_envelope()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("jsonl_review_event", text);
}

#[test]
fn format_stability_jsonl_manual_event() {
    let events = vec![manual_envelope()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("jsonl_manual_event", text);
}

// ============================================================================
// JSONL: mixed event types in one ledger
// ============================================================================

#[test]
fn format_stability_jsonl_mixed_events() {
    let events = vec![pr_envelope(), review_envelope(), manual_envelope()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("jsonl_mixed_events", text);
}

// ============================================================================
// Coverage manifest: full with multiple slices
// ============================================================================

#[test]
fn format_stability_coverage_manifest_full() {
    let manifest = CoverageManifest {
        run_id: RunId("stability_test_run".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: window(),
        mode: "merged".into(),
        sources: vec!["github".into(), "manual".into()],
        slices: vec![
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "author:octocat is:merged merged:2025-01-01..2025-02-01".into(),
                total_count: 15,
                fetched: 15,
                incomplete_results: Some(false),
                notes: vec![],
            },
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                },
                query: "author:octocat is:merged merged:2025-02-01..2025-03-01".into(),
                total_count: 100,
                fetched: 30,
                incomplete_results: Some(true),
                notes: vec![
                    "GitHub search API limit reached".into(),
                    "Results may be incomplete".into(),
                ],
            },
        ],
        warnings: vec!["Incomplete results for February slice".into()],
        completeness: Completeness::Partial,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &manifest).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_full", text);
}

// ============================================================================
// Coverage manifest: minimal
// ============================================================================

#[test]
fn format_stability_coverage_manifest_minimal() {
    let manifest = CoverageManifest {
        run_id: RunId("min".into()),
        generated_at: ts(),
        user: "u".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
        },
        mode: "created".into(),
        sources: vec![],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Unknown,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &manifest).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_minimal", text);
}

// ============================================================================
// Coverage manifest: edge cases
// ============================================================================

#[test]
fn format_stability_coverage_manifest_edge_cases() {
    let manifest = CoverageManifest {
        run_id: RunId("".into()),
        generated_at: ts(),
        user: "user with spaces & <special>".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
        },
        mode: "".into(),
        sources: vec!["".into(), "github".into(), "日本語ソース".into()],
        slices: vec![CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            },
            query: "query with \"quotes\" & <brackets>".into(),
            total_count: u64::MAX,
            fetched: 0,
            incomplete_results: None,
            notes: vec!["".into(), "Special chars: <>&\"'".into()],
        }],
        warnings: vec![
            "".into(),
            "Warning with émojis 🚀".into(),
            "Multi\nline\nwarning".into(),
        ],
        completeness: Completeness::Partial,
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &manifest).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_edge_cases", text);
}

// ============================================================================
// JSONL: edge-case event with special characters
// ============================================================================

#[test]
fn format_stability_jsonl_edge_cases() {
    let event = EventEnvelope {
        id: EventId::from_parts(["edge"]),
        kind: EventKind::PullRequest,
        occurred_at: ts(),
        actor: Actor {
            login: "user with spaces & <special>".into(),
            id: Some(u64::MAX),
        },
        repo: RepoRef {
            full_name: "".into(),
            html_url: Some("".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: u64::MAX,
            title: "título with émojis 🚀 & \"quotes\" <tags>".into(),
            state: PullRequestState::Merged,
            created_at: ts(),
            merged_at: Some(ts()),
            additions: Some(u64::MAX),
            deletions: Some(0),
            changed_files: Some(u64::MAX),
            touched_paths_hint: vec!["".into(), "path/with spaces/file.rs".into()],
            window: Some(window()),
        }),
        tags: vec!["".into(), "tag with spaces".into()],
        links: vec![Link {
            label: "".into(),
            url: "not-a-url".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Other("custom/system".into()),
            url: Some("".into()),
            opaque_id: Some("".into()),
        },
    };

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &[event]).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("jsonl_edge_cases", text);
}
