//! Integration tests for shiplog-redaction-projector.

use chrono::Utc;
use shiplog_alias::DeterministicAliasStore;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_policy::{redact_events_with_aliases, redact_workstreams_with_aliases};
use shiplog_redaction_projector::{
    parse_profile, project_events_with_aliases, project_workstreams_with_aliases,
};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn sample_events() -> Vec<EventEnvelope> {
    vec![
        EventEnvelope {
            id: EventId::from_parts(["integration", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "dev".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/private-repo".into(),
                html_url: Some("https://github.com/acme/private-repo".into()),
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 7,
                title: "Sensitive Title".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec!["secret/path.rs".into()],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: "https://github.com/acme/private-repo/pull/7".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/repos/acme/private-repo/pulls/7".into()),
                opaque_id: None,
            },
        },
        EventEnvelope {
            id: EventId::from_parts(["integration", "2"]),
            kind: EventKind::Manual,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "dev".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "acme/private-repo".into(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Incident,
                title: "Sensitive incident".into(),
                description: Some("Sensitive details".into()),
                started_at: None,
                ended_at: None,
                impact: Some("Sensitive impact".into()),
            }),
            tags: vec![],
            links: vec![Link {
                label: "incident".into(),
                url: "https://internal/wiki/incident".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: Some("https://internal/api/incidents/1".into()),
                opaque_id: None,
            },
        },
    ]
}

fn sample_workstreams() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "integration"]),
            title: "Sensitive Workstream".into(),
            summary: Some("Sensitive summary".into()),
            tags: vec!["repo".into(), "security".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    }
}

#[test]
fn projector_matches_policy_for_all_profile_inputs() {
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);
    let events = sample_events();
    let workstreams = sample_workstreams();

    for profile in ["internal", "manager", "public", "unexpected"] {
        let parsed = parse_profile(profile);

        let expected_events = redact_events_with_aliases(&events, parsed, &alias);
        let actual_events = project_events_with_aliases(&events, profile, &alias);
        assert_eq!(
            actual_events, expected_events,
            "events mismatch for {profile}"
        );

        let expected_workstreams = redact_workstreams_with_aliases(&workstreams, parsed, &alias);
        let actual_workstreams = project_workstreams_with_aliases(&workstreams, profile, &alias);
        assert_eq!(
            actual_workstreams, expected_workstreams,
            "workstreams mismatch for {profile}"
        );
    }
}

#[test]
fn unknown_profile_behaves_like_public() {
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);
    let events = sample_events();

    let unknown = project_events_with_aliases(&events, "team-only", &alias);
    let public = project_events_with_aliases(&events, "public", &alias);

    assert_eq!(unknown, public);
}
