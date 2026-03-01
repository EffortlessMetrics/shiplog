//! Format stability tests for shiplog-schema.
//!
//! Complements `snapshot.rs` with YAML snapshots, minimal structs (all optionals
//! set to `None`), and edge-case values to guard against accidental serialization
//! changes.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_schema::bundle::{BundleManifest, BundleProfile, FileChecksum};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

fn window() -> TimeWindow {
    TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }
}

// ============================================================================
// EventEnvelope — YAML format
// ============================================================================

#[test]
fn format_event_envelope_pr_yaml() {
    let event = EventEnvelope {
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
            touched_paths_hint: vec!["src/lib.rs".into()],
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
    };
    insta::assert_yaml_snapshot!(event);
}

// ============================================================================
// EventEnvelope — minimal (all optionals None, empty vecs)
// ============================================================================

#[test]
fn format_event_envelope_minimal_pr() {
    let event = EventEnvelope {
        id: EventId::from_parts(["min"]),
        kind: EventKind::PullRequest,
        occurred_at: ts(),
        actor: Actor {
            login: "a".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "x/y".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: "t".into(),
            state: PullRequestState::Open,
            created_at: ts(),
            merged_at: None,
            additions: None,
            deletions: None,
            changed_files: None,
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Unknown,
            url: None,
            opaque_id: None,
        },
    };
    insta::assert_json_snapshot!(event);
}

#[test]
fn format_event_envelope_minimal_review() {
    let event = EventEnvelope {
        id: EventId::from_parts(["min-review"]),
        kind: EventKind::Review,
        occurred_at: ts(),
        actor: Actor {
            login: "r".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "x/y".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 1,
            pull_title: "t".into(),
            submitted_at: ts(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Unknown,
            url: None,
            opaque_id: None,
        },
    };
    insta::assert_json_snapshot!(event);
}

#[test]
fn format_event_envelope_minimal_manual() {
    let event = EventEnvelope {
        id: EventId::from_parts(["min-manual"]),
        kind: EventKind::Manual,
        occurred_at: ts(),
        actor: Actor {
            login: "m".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "x/y".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: "n".into(),
            description: None,
            started_at: None,
            ended_at: None,
            impact: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Unknown,
            url: None,
            opaque_id: None,
        },
    };
    insta::assert_json_snapshot!(event);
}

// ============================================================================
// EventEnvelope — edge cases
// ============================================================================

#[test]
fn format_event_envelope_edge_cases() {
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
    insta::assert_json_snapshot!(event);
}

// ============================================================================
// ManualEventsFile — YAML format
// ============================================================================

#[test]
fn format_manual_events_file_full_yaml() {
    let file = ManualEventsFile {
        version: 1,
        generated_at: ts(),
        events: vec![ManualEventEntry {
            id: "incident-1".into(),
            event_type: ManualEventType::Incident,
            date: ManualDate::Range {
                start: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
                end: NaiveDate::from_ymd_opt(2025, 3, 16).unwrap(),
            },
            title: "Production outage response".into(),
            description: Some("Responded to production outage affecting payments".into()),
            workstream: Some("infrastructure".into()),
            tags: vec!["incident".into(), "oncall".into()],
            receipts: vec![Link {
                label: "postmortem".into(),
                url: "https://wiki.internal/p42".into(),
            }],
            impact: Some("Reduced MTTR by 50%".into()),
        }],
    };
    insta::assert_yaml_snapshot!(file);
}

#[test]
fn format_manual_events_file_minimal_yaml() {
    let file = ManualEventsFile {
        version: 1,
        generated_at: ts(),
        events: vec![ManualEventEntry {
            id: "note-1".into(),
            event_type: ManualEventType::Note,
            date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap()),
            title: "Simple note".into(),
            description: None,
            workstream: None,
            tags: vec![],
            receipts: vec![],
            impact: None,
        }],
    };
    insta::assert_yaml_snapshot!(file);
}

// ============================================================================
// ManualDate variants
// ============================================================================

#[test]
fn format_manual_date_single() {
    let d = ManualDate::Single(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    insta::assert_json_snapshot!(d);
}

#[test]
fn format_manual_date_range() {
    let d = ManualDate::Range {
        start: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        end: NaiveDate::from_ymd_opt(2025, 12, 31).unwrap(),
    };
    insta::assert_json_snapshot!(d);
}

// ============================================================================
// WorkstreamsFile — YAML format with edge cases
// ============================================================================

#[test]
fn format_workstreams_file_yaml() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![
            Workstream {
                id: WorkstreamId::from_parts(["ws", "auth"]),
                title: "Authentication".into(),
                summary: Some("OAuth2 integration".into()),
                tags: vec!["security".into(), "backend".into()],
                stats: WorkstreamStats {
                    pull_requests: 5,
                    reviews: 3,
                    manual_events: 1,
                },
                events: vec![
                    EventId::from_parts(["github", "pr", "acme/widgets", "42"]),
                    EventId::from_parts(["github", "pr", "acme/widgets", "43"]),
                ],
                receipts: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
            },
            Workstream {
                id: WorkstreamId::from_parts(["ws", "infra"]),
                title: "Infrastructure".into(),
                summary: None,
                tags: vec![],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            },
        ],
    };
    insta::assert_yaml_snapshot!(ws);
}

// ============================================================================
// CoverageManifest — YAML format with multiple slices
// ============================================================================

#[test]
fn format_coverage_manifest_yaml() {
    let manifest = CoverageManifest {
        run_id: RunId("test_run_456".into()),
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
                query: "author:octocat is:merged".into(),
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
                query: "author:octocat is:merged".into(),
                total_count: 50,
                fetched: 30,
                incomplete_results: Some(true),
                notes: vec!["Rate limited".into()],
            },
        ],
        warnings: vec!["Incomplete results for February".into()],
        completeness: Completeness::Partial,
    };
    insta::assert_yaml_snapshot!(manifest);
}

// ============================================================================
// BundleManifest — YAML format with multiple files
// ============================================================================

#[test]
fn format_bundle_manifest_yaml() {
    let manifest = BundleManifest {
        run_id: RunId("test_run_789".into()),
        generated_at: ts(),
        profile: BundleProfile::Manager,
        files: vec![
            FileChecksum {
                path: "packet.md".into(),
                sha256: "abc123def456".into(),
                bytes: 4096,
            },
            FileChecksum {
                path: "profiles/manager/packet.md".into(),
                sha256: "789abc012def".into(),
                bytes: 2048,
            },
        ],
    };
    insta::assert_yaml_snapshot!(manifest);
}

// ============================================================================
// Edge-case composites
// ============================================================================

#[test]
fn format_coverage_manifest_empty() {
    let manifest = CoverageManifest {
        run_id: RunId("empty".into()),
        generated_at: ts(),
        user: "".into(),
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
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn format_bundle_manifest_empty_files() {
    let manifest = BundleManifest {
        run_id: RunId("empty".into()),
        generated_at: ts(),
        profile: BundleProfile::Internal,
        files: vec![],
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn format_file_checksum_edge_cases() {
    let checksums = vec![
        FileChecksum {
            path: "".into(),
            sha256: "".into(),
            bytes: 0,
        },
        FileChecksum {
            path: "deeply/nested/path/to/file with spaces.txt".into(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".into(),
            bytes: u64::MAX,
        },
    ];
    insta::assert_json_snapshot!(checksums);
}

#[test]
fn format_workstream_stats_zero() {
    insta::assert_json_snapshot!(WorkstreamStats::zero());
}
