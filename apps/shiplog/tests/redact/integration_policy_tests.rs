//! Integration tests for shiplog::redact facade policy behavior.

use chrono::Utc;
use shiplog::ids::{EventId, WorkstreamId};
use shiplog::ports::Redactor;
use shiplog::redact::{DeterministicRedactor, RedactionProfile};
use shiplog::schema::event::*;
use shiplog::schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

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
                opaque_id: Some("github-pr-node-id".into()),
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
                opaque_id: Some("manual-incident-1".into()),
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
fn profile_reexport_parses_canonical_names() {
    assert_eq!(
        RedactionProfile::from_profile_str("internal"),
        RedactionProfile::Internal
    );
    assert_eq!(
        RedactionProfile::from_profile_str("manager"),
        RedactionProfile::Manager
    );
    assert_eq!(
        RedactionProfile::from_profile_str("public"),
        RedactionProfile::Public
    );
    assert_eq!(
        RedactionProfile::from_profile_str("unknown"),
        RedactionProfile::Public
    );
}

#[test]
fn redactor_event_profiles_apply_facade_contract() {
    let redactor = DeterministicRedactor::new(b"integration-key");
    let events = sample_events();

    let internal = redactor
        .redact_events(&events, "internal")
        .expect("internal profile succeeds");
    assert_eq!(internal, events);

    let manager = redactor
        .redact_events(&events, "manager")
        .expect("manager profile succeeds");
    assert_eq!(manager[0].repo.full_name, "acme/private-repo");
    assert!(manager[0].links.is_empty());
    match &manager[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "Sensitive PR");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected pull request"),
    }
    match &manager[1].payload {
        EventPayload::Manual(manual) => {
            assert_eq!(manual.title, "Sensitive Incident");
            assert!(manual.description.is_none());
            assert!(manual.impact.is_none());
        }
        _ => panic!("expected manual event"),
    }

    let public = redactor
        .redact_events(&events, "public")
        .expect("public profile succeeds");
    assert_ne!(public[0].repo.full_name, "acme/private-repo");
    assert!(public[0].repo.full_name.starts_with("repo-"));
    assert!(public[0].repo.html_url.is_none());
    assert!(public[0].links.is_empty());
    assert!(public[0].source.url.is_none());
    assert!(public[0].source.opaque_id.is_none());
    assert!(public[1].source.opaque_id.is_none());
    match &public[0].payload {
        EventPayload::PullRequest(pr) => {
            assert_eq!(pr.title, "[redacted]");
            assert!(pr.touched_paths_hint.is_empty());
        }
        _ => panic!("expected pull request"),
    }
    match &public[1].payload {
        EventPayload::Manual(manual) => {
            assert_eq!(manual.title, "[redacted]");
            assert!(manual.description.is_none());
            assert!(manual.impact.is_none());
        }
        _ => panic!("expected manual event"),
    }
}

#[test]
fn redactor_workstream_profiles_apply_facade_contract() {
    let redactor = DeterministicRedactor::new(b"integration-key");
    let workstreams = sample_workstreams();

    let internal = redactor
        .redact_workstreams(&workstreams, "internal")
        .expect("internal profile succeeds");
    assert_eq!(internal, workstreams);

    let manager = redactor
        .redact_workstreams(&workstreams, "manager")
        .expect("manager profile succeeds");
    let manager_ws = &manager.workstreams[0];
    assert_eq!(manager_ws.title, "Sensitive Workstream");
    assert!(manager_ws.summary.is_none());
    assert!(manager_ws.tags.contains(&"repo".to_string()));

    let public = redactor
        .redact_workstreams(&workstreams, "public")
        .expect("public profile succeeds");
    let public_ws = &public.workstreams[0];
    assert_ne!(public_ws.title, "Sensitive Workstream");
    assert!(public_ws.title.starts_with("ws-"));
    assert!(public_ws.summary.is_none());
    assert!(!public_ws.tags.contains(&"repo".to_string()));
}
