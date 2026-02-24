//! Integration tests ensuring shiplog-redact delegates to shiplog-redaction-policy.

use chrono::Utc;
use shiplog_alias::DeterministicAliasStore;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::Redactor;
use shiplog_redaction_policy::{
    RedactionProfile, redact_events_with_aliases, redact_workstreams_with_aliases,
};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn sample_events() -> Vec<EventEnvelope> {
    vec![
        EventEnvelope {
            id: EventId::from_parts(["integration", "event", "1"]),
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
                number: 1,
                title: "Sensitive PR".into(),
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
                url: "https://github.com/acme/private-repo/pull/1".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/repos/acme/private-repo/pulls/1".into()),
                opaque_id: None,
            },
        },
        EventEnvelope {
            id: EventId::from_parts(["integration", "event", "2"]),
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
                title: "Sensitive Incident".into(),
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
fn redactor_event_outputs_match_extracted_policy_for_all_profiles() {
    let redactor = shiplog_redact::DeterministicRedactor::new(b"integration-key");
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);
    let events = sample_events();

    for profile in ["internal", "manager", "public"] {
        let expected = redact_events_with_aliases(
            &events,
            RedactionProfile::from_profile_str(profile),
            &alias,
        );
        let actual = redactor
            .redact_events(&events, profile)
            .expect("redact events should succeed");
        assert_eq!(actual, expected, "profile mismatch for {profile}");
    }
}

#[test]
fn redactor_workstream_outputs_match_extracted_policy_for_all_profiles() {
    let redactor = shiplog_redact::DeterministicRedactor::new(b"integration-key");
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);
    let workstreams = sample_workstreams();

    for profile in ["internal", "manager", "public"] {
        let expected = redact_workstreams_with_aliases(
            &workstreams,
            RedactionProfile::from_profile_str(profile),
            &alias,
        );
        let actual = redactor
            .redact_workstreams(&workstreams, profile)
            .expect("redact workstreams should succeed");
        assert_eq!(actual, expected, "profile mismatch for {profile}");
    }
}
