//! Snapshot tests for shiplog-redaction-projector projection outputs.

use chrono::{TimeZone, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_projector::{project_events_with_aliases, project_workstreams_with_aliases};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn stable_alias(kind: &str, value: &str) -> String {
    format!("{kind}-REDACTED({value})")
}

fn fixed_events() -> Vec<EventEnvelope> {
    let t = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    vec![
        EventEnvelope {
            id: EventId::from_parts(["snap", "pr", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: t,
            actor: Actor {
                login: "dev".into(),
                id: Some(1),
            },
            repo: RepoRef {
                full_name: "acme/core".into(),
                html_url: Some("https://github.com/acme/core".into()),
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 10,
                title: "Add secret feature".into(),
                state: PullRequestState::Merged,
                created_at: t,
                merged_at: Some(t),
                additions: Some(50),
                deletions: Some(5),
                changed_files: Some(3),
                touched_paths_hint: vec!["src/secret.rs".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/acme/core/pull/10".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/repos/acme/core/pulls/10".into()),
                opaque_id: None,
            },
        },
        EventEnvelope {
            id: EventId::from_parts(["snap", "manual", "1"]),
            kind: EventKind::Manual,
            occurred_at: t,
            actor: Actor {
                login: "dev".into(),
                id: Some(1),
            },
            repo: RepoRef {
                full_name: "acme/core".into(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Design,
                title: "Architecture review".into(),
                description: Some("Confidential design discussion".into()),
                started_at: None,
                ended_at: None,
                impact: Some("Shaped Q3 roadmap".into()),
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: None,
                opaque_id: None,
            },
        },
    ]
}

fn fixed_workstreams() -> WorkstreamsFile {
    let t = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    WorkstreamsFile {
        version: 1,
        generated_at: t,
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "snap"]),
            title: "Secret Initiative".into(),
            summary: Some("Confidential summary".into()),
            tags: vec!["repo".into(), "infra".into()],
            stats: WorkstreamStats {
                pull_requests: 1,
                reviews: 0,
                manual_events: 1,
            },
            events: vec![],
            receipts: vec![],
        }],
    }
}

#[test]
fn snapshot_project_events_public() {
    let events = fixed_events();
    let out = project_events_with_aliases(&events, "public", &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("project_events_public", json);
}

#[test]
fn snapshot_project_events_manager() {
    let events = fixed_events();
    let out = project_events_with_aliases(&events, "manager", &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("project_events_manager", json);
}

#[test]
fn snapshot_project_events_internal() {
    let events = fixed_events();
    let out = project_events_with_aliases(&events, "internal", &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("project_events_internal", json);
}

#[test]
fn snapshot_project_workstreams_public() {
    let ws = fixed_workstreams();
    let out = project_workstreams_with_aliases(&ws, "public", &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("project_workstreams_public", json);
}

#[test]
fn snapshot_project_workstreams_manager() {
    let ws = fixed_workstreams();
    let out = project_workstreams_with_aliases(&ws, "manager", &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("project_workstreams_manager", json);
}

#[test]
fn snapshot_project_unknown_is_public() {
    let events = fixed_events();
    let unknown = project_events_with_aliases(&events, "unknown-profile", &stable_alias);
    let public = project_events_with_aliases(&events, "public", &stable_alias);
    assert_eq!(
        serde_json::to_string(&unknown).unwrap(),
        serde_json::to_string(&public).unwrap(),
    );
}
