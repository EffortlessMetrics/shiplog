//! Snapshot tests for shiplog-schema serialization formats.
//!
//! Uses insta to pin the JSON representation of each key type, catching
//! accidental serialization-breaking changes.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_schema::bundle::{BundleManifest, BundleProfile, FileChecksum};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// Fixed timestamp for deterministic snapshots.
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
// EventKind variant snapshots
// ============================================================================

#[test]
fn snapshot_event_kind_pull_request() {
    insta::assert_json_snapshot!(EventKind::PullRequest, @r#""PullRequest""#);
}

#[test]
fn snapshot_event_kind_review() {
    insta::assert_json_snapshot!(EventKind::Review, @r#""Review""#);
}

#[test]
fn snapshot_event_kind_manual() {
    insta::assert_json_snapshot!(EventKind::Manual, @r#""Manual""#);
}

// ============================================================================
// SourceSystem variant snapshots
// ============================================================================

#[test]
fn snapshot_source_system_github() {
    insta::assert_json_snapshot!(SourceSystem::Github, @r#""github""#);
}

#[test]
fn snapshot_source_system_json_import() {
    insta::assert_json_snapshot!(SourceSystem::JsonImport, @r#""json_import""#);
}

#[test]
fn snapshot_source_system_local_git() {
    insta::assert_json_snapshot!(SourceSystem::LocalGit, @r#""local_git""#);
}

#[test]
fn snapshot_source_system_manual() {
    insta::assert_json_snapshot!(SourceSystem::Manual, @r#""manual""#);
}

#[test]
fn snapshot_source_system_unknown() {
    insta::assert_json_snapshot!(SourceSystem::Unknown, @r#""unknown""#);
}

#[test]
fn snapshot_source_system_other() {
    insta::assert_json_snapshot!(SourceSystem::Other("gitlab".into()), @r#""gitlab""#);
}

// ============================================================================
// PullRequestState variant snapshots
// ============================================================================

#[test]
fn snapshot_pr_state_open() {
    insta::assert_json_snapshot!(PullRequestState::Open, @r#""Open""#);
}

#[test]
fn snapshot_pr_state_closed() {
    insta::assert_json_snapshot!(PullRequestState::Closed, @r#""Closed""#);
}

#[test]
fn snapshot_pr_state_merged() {
    insta::assert_json_snapshot!(PullRequestState::Merged, @r#""Merged""#);
}

#[test]
fn snapshot_pr_state_unknown() {
    insta::assert_json_snapshot!(PullRequestState::Unknown, @r#""Unknown""#);
}

// ============================================================================
// ManualEventType variant snapshots
// ============================================================================

#[test]
fn snapshot_manual_event_type_note() {
    insta::assert_json_snapshot!(ManualEventType::Note, @r#""Note""#);
}

#[test]
fn snapshot_manual_event_type_incident() {
    insta::assert_json_snapshot!(ManualEventType::Incident, @r#""Incident""#);
}

#[test]
fn snapshot_manual_event_type_design() {
    insta::assert_json_snapshot!(ManualEventType::Design, @r#""Design""#);
}

#[test]
fn snapshot_manual_event_type_mentoring() {
    insta::assert_json_snapshot!(ManualEventType::Mentoring, @r#""Mentoring""#);
}

#[test]
fn snapshot_manual_event_type_launch() {
    insta::assert_json_snapshot!(ManualEventType::Launch, @r#""Launch""#);
}

#[test]
fn snapshot_manual_event_type_migration() {
    insta::assert_json_snapshot!(ManualEventType::Migration, @r#""Migration""#);
}

#[test]
fn snapshot_manual_event_type_review() {
    insta::assert_json_snapshot!(ManualEventType::Review, @r#""Review""#);
}

#[test]
fn snapshot_manual_event_type_other() {
    insta::assert_json_snapshot!(ManualEventType::Other, @r#""Other""#);
}

// ============================================================================
// RepoVisibility variant snapshots
// ============================================================================

#[test]
fn snapshot_repo_visibility_public() {
    insta::assert_json_snapshot!(RepoVisibility::Public, @r#""Public""#);
}

#[test]
fn snapshot_repo_visibility_private() {
    insta::assert_json_snapshot!(RepoVisibility::Private, @r#""Private""#);
}

#[test]
fn snapshot_repo_visibility_unknown() {
    insta::assert_json_snapshot!(RepoVisibility::Unknown, @r#""Unknown""#);
}

// ============================================================================
// BundleProfile variant snapshots
// ============================================================================

#[test]
fn snapshot_bundle_profile_internal() {
    insta::assert_json_snapshot!(BundleProfile::Internal, @r#""Internal""#);
}

#[test]
fn snapshot_bundle_profile_manager() {
    insta::assert_json_snapshot!(BundleProfile::Manager, @r#""Manager""#);
}

#[test]
fn snapshot_bundle_profile_public() {
    insta::assert_json_snapshot!(BundleProfile::Public, @r#""Public""#);
}

// ============================================================================
// Completeness variant snapshots
// ============================================================================

#[test]
fn snapshot_completeness_complete() {
    insta::assert_json_snapshot!(Completeness::Complete, @r#""Complete""#);
}

#[test]
fn snapshot_completeness_partial() {
    insta::assert_json_snapshot!(Completeness::Partial, @r#""Partial""#);
}

#[test]
fn snapshot_completeness_unknown() {
    insta::assert_json_snapshot!(Completeness::Unknown, @r#""Unknown""#);
}

// ============================================================================
// Full EventEnvelope snapshots – one per EventPayload variant
// ============================================================================

#[test]
fn snapshot_event_envelope_pull_request() {
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
    insta::assert_json_snapshot!(event);
}

#[test]
fn snapshot_event_envelope_review() {
    let event = EventEnvelope {
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
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };
    insta::assert_json_snapshot!(event);
}

#[test]
fn snapshot_event_envelope_manual() {
    let event = EventEnvelope {
        id: EventId::from_parts(["manual", "acme/widgets", "incident-response"]),
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
            description: Some("Responded to production outage".into()),
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
    };
    insta::assert_json_snapshot!(event);
}

// ============================================================================
// Backward-compatible format snapshots
// ============================================================================

#[test]
fn snapshot_source_system_backward_compat_object_form() {
    // Old enum-style tagged objects should deserialise to the expected variant.
    let json = r#"{"Github":null}"#;
    let ss: SourceSystem = serde_json::from_str(json).unwrap();
    insta::assert_json_snapshot!(ss, @r#""github""#);
}

#[test]
fn snapshot_source_system_backward_compat_other_object() {
    let json = r#"{"Other":"gitlab"}"#;
    let ss: SourceSystem = serde_json::from_str(json).unwrap();
    insta::assert_json_snapshot!(ss, @r#""gitlab""#);
}

// ============================================================================
// Composite type snapshots
// ============================================================================

#[test]
fn snapshot_time_window() {
    insta::assert_json_snapshot!(window());
}

#[test]
fn snapshot_coverage_manifest() {
    let manifest = CoverageManifest {
        run_id: shiplog_ids::RunId("test_run_123".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: window(),
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![CoverageSlice {
            window: window(),
            query: "author:octocat is:merged".into(),
            total_count: 42,
            fetched: 42,
            incomplete_results: Some(false),
            notes: vec![],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn snapshot_bundle_manifest() {
    let manifest = BundleManifest {
        run_id: shiplog_ids::RunId("test_run_123".into()),
        generated_at: ts(),
        profile: BundleProfile::Internal,
        files: vec![FileChecksum {
            path: "packet.md".into(),
            sha256: "abc123".into(),
            bytes: 4096,
        }],
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn snapshot_workstreams_file() {
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "auth"]),
            title: "Authentication".into(),
            summary: Some("OAuth2 integration".into()),
            tags: vec!["security".into()],
            stats: WorkstreamStats {
                pull_requests: 3,
                reviews: 1,
                manual_events: 0,
            },
            events: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
            receipts: vec![EventId::from_parts(["github", "pr", "acme/widgets", "42"])],
        }],
    };
    insta::assert_json_snapshot!(ws);
}
