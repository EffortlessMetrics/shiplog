//! Snapshot tests for shiplog-redaction-policy redaction outputs.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_policy::{
    RedactionProfile, redact_event_with_aliases, redact_workstream_with_aliases,
};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats};

fn stable_alias(kind: &str, value: &str) -> String {
    format!("{kind}-REDACTED({value})")
}

fn fixed_pr_event() -> EventEnvelope {
    let fixed_time = Utc.with_ymd_and_hms(2025, 6, 15, 12, 0, 0).unwrap();
    EventEnvelope {
        id: EventId::from_parts(["snap", "pr", "1"]),
        kind: EventKind::PullRequest,
        occurred_at: fixed_time,
        actor: Actor {
            login: "developer".into(),
            id: Some(42),
        },
        repo: RepoRef {
            full_name: "acme/secret-project".into(),
            html_url: Some("https://github.com/acme/secret-project".into()),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 99,
            title: "Implement confidential feature X".into(),
            state: PullRequestState::Merged,
            created_at: fixed_time,
            merged_at: Some(fixed_time),
            additions: Some(150),
            deletions: Some(30),
            changed_files: Some(8),
            touched_paths_hint: vec!["src/secret/module.rs".into(), "config/keys.toml".into()],
            window: None,
        }),
        tags: vec!["feature".into(), "priority".into()],
        links: vec![
            Link {
                label: "pr".into(),
                url: "https://github.com/acme/secret-project/pull/99".into(),
            },
            Link {
                label: "issue".into(),
                url: "https://github.com/acme/secret-project/issues/50".into(),
            },
        ],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/repos/acme/secret-project/pulls/99".into()),
            opaque_id: None,
        },
    }
}

fn fixed_review_event() -> EventEnvelope {
    let fixed_time = Utc.with_ymd_and_hms(2025, 6, 15, 14, 0, 0).unwrap();
    EventEnvelope {
        id: EventId::from_parts(["snap", "review", "1"]),
        kind: EventKind::Review,
        occurred_at: fixed_time,
        actor: Actor {
            login: "reviewer".into(),
            id: Some(7),
        },
        repo: RepoRef {
            full_name: "acme/secret-project".into(),
            html_url: Some("https://github.com/acme/secret-project".into()),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 99,
            pull_title: "Implement confidential feature X".into(),
            submitted_at: fixed_time,
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "review".into(),
            url: "https://github.com/acme/secret-project/pull/99#pullrequestreview-1".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/repos/acme/secret-project/pulls/99/reviews/1".into()),
            opaque_id: None,
        },
    }
}

fn fixed_manual_event() -> EventEnvelope {
    let fixed_time = Utc.with_ymd_and_hms(2025, 6, 16, 9, 0, 0).unwrap();
    EventEnvelope {
        id: EventId::from_parts(["snap", "manual", "1"]),
        kind: EventKind::Manual,
        occurred_at: fixed_time,
        actor: Actor {
            login: "developer".into(),
            id: Some(42),
        },
        repo: RepoRef {
            full_name: "acme/secret-project".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "Production database outage".into(),
            description: Some("Root cause: misconfigured connection pool in secret-service".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 6, 16).unwrap()),
            impact: Some("3 hours downtime affecting 500 enterprise customers".into()),
        }),
        tags: vec!["incident".into()],
        links: vec![Link {
            label: "postmortem".into(),
            url: "https://internal.wiki/postmortem/2025-06-16".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: Some("https://internal.api/incidents/42".into()),
            opaque_id: None,
        },
    }
}

fn fixed_workstream() -> Workstream {
    Workstream {
        id: WorkstreamId::from_parts(["ws", "snapshot"]),
        title: "Secret Platform Migration".into(),
        summary: Some("Migrating internal auth from legacy LDAP to OAuth2".into()),
        tags: vec!["platform".into(), "repo".into(), "security".into()],
        stats: WorkstreamStats {
            pull_requests: 5,
            reviews: 3,
            manual_events: 1,
        },
        events: vec![EventId::from_parts(["snap", "pr", "1"])],
        receipts: vec![EventId::from_parts(["snap", "pr", "1"])],
    }
}

#[test]
fn snapshot_pr_event_internal_profile() {
    let event = fixed_pr_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Internal, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("pr_event_internal", json);
}

#[test]
fn snapshot_pr_event_manager_profile() {
    let event = fixed_pr_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Manager, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("pr_event_manager", json);
}

#[test]
fn snapshot_pr_event_public_profile() {
    let event = fixed_pr_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("pr_event_public", json);
}

#[test]
fn snapshot_review_event_public_profile() {
    let event = fixed_review_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("review_event_public", json);
}

#[test]
fn snapshot_manual_event_manager_profile() {
    let event = fixed_manual_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Manager, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("manual_event_manager", json);
}

#[test]
fn snapshot_manual_event_public_profile() {
    let event = fixed_manual_event();
    let out = redact_event_with_aliases(event, RedactionProfile::Public, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("manual_event_public", json);
}

#[test]
fn snapshot_workstream_internal_profile() {
    let ws = fixed_workstream();
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Internal, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("workstream_internal", json);
}

#[test]
fn snapshot_workstream_manager_profile() {
    let ws = fixed_workstream();
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Manager, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("workstream_manager", json);
}

#[test]
fn snapshot_workstream_public_profile() {
    let ws = fixed_workstream();
    let out = redact_workstream_with_aliases(ws, RedactionProfile::Public, &stable_alias);
    let json = serde_json::to_string_pretty(&out).unwrap();
    insta::assert_snapshot!("workstream_public", json);
}
