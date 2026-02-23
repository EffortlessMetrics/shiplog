//! Profile-based structural redaction policy for shiplog.
//!
//! This crate holds only redaction policy rules:
//! - profile parsing (`internal`, `manager`, `public`)
//! - field-level transforms for events and workstreams
//! - alias resolver abstraction used by `public` profile

use serde::{Deserialize, Serialize};
use shiplog_schema::event::{EventEnvelope, EventPayload, RepoRef, RepoVisibility};
use shiplog_schema::workstream::{Workstream, WorkstreamsFile};

/// Rendering profiles.
///
/// The tool produces multiple projections from the same ledger.
/// Think of them as lenses, not forks.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum RedactionProfile {
    Internal,
    Manager,
    Public,
}

impl RedactionProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            RedactionProfile::Internal => "internal",
            RedactionProfile::Manager => "manager",
            RedactionProfile::Public => "public",
        }
    }

    /// Parse profile string, defaulting unknown values to `Public`.
    pub fn from_profile_str(profile: &str) -> Self {
        match profile {
            "internal" => RedactionProfile::Internal,
            "manager" => RedactionProfile::Manager,
            _ => RedactionProfile::Public,
        }
    }
}

/// Alias resolver used by public redaction.
pub trait AliasResolver {
    fn alias(&self, kind: &str, value: &str) -> String;
}

impl<F> AliasResolver for F
where
    F: Fn(&str, &str) -> String,
{
    fn alias(&self, kind: &str, value: &str) -> String {
        (self)(kind, value)
    }
}

fn redact_repo_public<A: AliasResolver + ?Sized>(repo: &RepoRef, aliases: &A) -> RepoRef {
    RepoRef {
        full_name: aliases.alias("repo", &repo.full_name),
        html_url: None,
        visibility: RepoVisibility::Unknown,
    }
}

/// Redact a single event for the selected profile.
pub fn redact_event_with_aliases<A: AliasResolver + ?Sized>(
    mut event: EventEnvelope,
    profile: RedactionProfile,
    aliases: &A,
) -> EventEnvelope {
    match profile {
        RedactionProfile::Internal => event,
        RedactionProfile::Manager => {
            match &mut event.payload {
                EventPayload::PullRequest(pr) => {
                    pr.touched_paths_hint.clear();
                }
                EventPayload::Review(_) => {}
                EventPayload::Manual(m) => {
                    m.description = None;
                    m.impact = None;
                }
            }

            event.links.clear();
            event
        }
        RedactionProfile::Public => {
            event.repo = redact_repo_public(&event.repo, aliases);

            match &mut event.payload {
                EventPayload::PullRequest(pr) => {
                    pr.title = "[redacted]".to_string();
                    pr.touched_paths_hint.clear();
                }
                EventPayload::Review(r) => {
                    r.pull_title = "[redacted]".to_string();
                }
                EventPayload::Manual(m) => {
                    m.title = "[redacted]".to_string();
                    m.description = None;
                    m.impact = None;
                }
            }

            event.links.clear();
            event.source.url = None;
            event
        }
    }
}

/// Redact all events for the selected profile.
pub fn redact_events_with_aliases<A: AliasResolver + ?Sized>(
    events: &[EventEnvelope],
    profile: RedactionProfile,
    aliases: &A,
) -> Vec<EventEnvelope> {
    match profile {
        RedactionProfile::Internal => events.to_vec(),
        _ => events
            .iter()
            .cloned()
            .map(|event| redact_event_with_aliases(event, profile, aliases))
            .collect(),
    }
}

/// Redact a single workstream for the selected profile.
pub fn redact_workstream_with_aliases<A: AliasResolver + ?Sized>(
    mut workstream: Workstream,
    profile: RedactionProfile,
    aliases: &A,
) -> Workstream {
    match profile {
        RedactionProfile::Internal => workstream,
        RedactionProfile::Manager => {
            workstream.summary = None;
            workstream
        }
        RedactionProfile::Public => {
            workstream.title = aliases.alias("ws", &workstream.title);
            workstream.summary = None;
            workstream.tags.retain(|tag| tag != "repo");
            workstream
        }
    }
}

/// Redact all workstreams for the selected profile.
pub fn redact_workstreams_with_aliases<A: AliasResolver + ?Sized>(
    workstreams: &WorkstreamsFile,
    profile: RedactionProfile,
    aliases: &A,
) -> WorkstreamsFile {
    match profile {
        RedactionProfile::Internal => workstreams.clone(),
        _ => WorkstreamsFile {
            workstreams: workstreams
                .workstreams
                .iter()
                .cloned()
                .map(|workstream| redact_workstream_with_aliases(workstream, profile, aliases))
                .collect(),
            ..workstreams.clone()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use shiplog_ids::{EventId, WorkstreamId};
    use shiplog_schema::event::*;
    use shiplog_schema::workstream::WorkstreamStats;

    fn alias(kind: &str, value: &str) -> String {
        let mut acc = 14695981039346656037u64;
        for byte in kind.bytes().chain(value.bytes()) {
            acc ^= u64::from(byte);
            acc = acc.wrapping_mul(1099511628211);
        }
        format!("{kind}-{acc:016x}")
    }

    fn sample_pr_event() -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["x", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "a".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "org/repo".into(),
                html_url: Some("https://github.com/org/repo".into()),
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "Highly Sensitive".into(),
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
                url: "https://github.com/org/repo/pull/1".into(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/repos/org/repo/pulls/1".into()),
                opaque_id: None,
            },
        }
    }

    fn sample_workstreams() -> WorkstreamsFile {
        WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![Workstream {
                id: WorkstreamId::from_parts(["ws", "a"]),
                title: "Sensitive Platform Migration".into(),
                summary: Some("Internal architectural details".into()),
                tags: vec!["platform".into(), "repo".into()],
                stats: WorkstreamStats::zero(),
                events: vec![],
                receipts: vec![],
            }],
        }
    }

    #[test]
    fn profile_parser_defaults_unknown_to_public() {
        assert_eq!(
            RedactionProfile::from_profile_str("unexpected"),
            RedactionProfile::Public
        );
    }

    #[test]
    fn public_profile_redacts_pr_sensitive_fields() {
        let event = sample_pr_event();
        let out = redact_event_with_aliases(event, RedactionProfile::Public, &alias);

        assert!(out.links.is_empty());
        assert!(out.source.url.is_none());
        assert_ne!(out.repo.full_name, "org/repo");

        match out.payload {
            EventPayload::PullRequest(pr) => {
                assert_eq!(pr.title, "[redacted]");
                assert!(pr.touched_paths_hint.is_empty());
            }
            _ => panic!("expected pull request payload"),
        }
    }

    #[test]
    fn manager_profile_preserves_context_and_removes_detail_fields() {
        let mut event = sample_pr_event();
        event.payload = EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "Database incident".into(),
            description: Some("Sensitive details".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            impact: Some("Sensitive impact".into()),
        });

        let out = redact_event_with_aliases(event, RedactionProfile::Manager, &alias);
        match out.payload {
            EventPayload::Manual(m) => {
                assert_eq!(m.title, "Database incident");
                assert!(m.description.is_none());
                assert!(m.impact.is_none());
            }
            _ => panic!("expected manual payload"),
        }
    }

    #[test]
    fn workstream_public_profile_aliases_title_and_filters_repo_tag() {
        let ws_file = sample_workstreams();
        let out = redact_workstreams_with_aliases(&ws_file, RedactionProfile::Public, &alias);
        let ws = &out.workstreams[0];

        assert_ne!(ws.title, "Sensitive Platform Migration");
        assert!(ws.summary.is_none());
        assert!(!ws.tags.contains(&"repo".to_string()));
    }

    #[test]
    fn workstream_manager_profile_removes_summary_only() {
        let ws_file = sample_workstreams();
        let out = redact_workstreams_with_aliases(&ws_file, RedactionProfile::Manager, &alias);
        let ws = &out.workstreams[0];

        assert_eq!(ws.title, "Sensitive Platform Migration");
        assert!(ws.summary.is_none());
        assert!(ws.tags.contains(&"repo".to_string()));
    }

    #[test]
    fn closure_alias_resolver_is_supported() {
        let resolver = |kind: &str, value: &str| format!("{kind}:{value}");
        assert_eq!(resolver.alias("repo", "org/repo"), "repo:org/repo");
    }
}
