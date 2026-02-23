//! Integration tests for shiplog-redaction-policy with shiplog-alias.

use chrono::Utc;
use shiplog_alias::DeterministicAliasStore;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_redaction_policy::{
    RedactionProfile, redact_events_with_aliases, redact_workstreams_with_aliases,
};
use shiplog_redaction_profile::RedactionProfile as ProfileCrateProfile;
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["integration", &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(10),
            deletions: Some(3),
            changed_files: Some(2),
            touched_paths_hint: vec!["secret/path.rs".into()],
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/{repo}/pulls/{number}"
            )),
            opaque_id: None,
        },
    }
}

#[test]
fn public_projection_uses_alias_store_deterministically() {
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);

    let events = vec![
        pr_event("acme/private-repo", 1, "secret 1"),
        pr_event("acme/private-repo", 2, "secret 2"),
    ];

    let redacted = redact_events_with_aliases(&events, RedactionProfile::Public, &alias);
    let expected_repo_alias = alias_store.alias("repo", "acme/private-repo");

    assert_eq!(redacted[0].repo.full_name, expected_repo_alias);
    assert_eq!(redacted[1].repo.full_name, expected_repo_alias);
}

#[test]
fn workstream_public_projection_shares_alias_contract_with_alias_store() {
    let alias_store = DeterministicAliasStore::new(b"integration-key");
    let alias = |kind: &str, value: &str| alias_store.alias(kind, value);

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "integration"]),
            title: "Highly Confidential Workstream".into(),
            summary: Some("Hidden details".into()),
            tags: vec!["repo".into(), "security".into()],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };

    let redacted = redact_workstreams_with_aliases(&workstreams, RedactionProfile::Public, &alias);
    let expected_alias = alias_store.alias("ws", "Highly Confidential Workstream");

    assert_eq!(redacted.workstreams[0].title, expected_alias);
    assert!(redacted.workstreams[0].summary.is_none());
    assert!(!redacted.workstreams[0].tags.contains(&"repo".to_string()));
}

#[test]
fn policy_reexports_profile_type_from_profile_crate() {
    let profile: RedactionProfile = ProfileCrateProfile::Manager;
    assert_eq!(profile.as_str(), "manager");
    assert_eq!(
        RedactionProfile::from_profile_str("unexpected"),
        ProfileCrateProfile::Public
    );
}
