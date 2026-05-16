//! Pure assembler for a PR [`EventEnvelope`]. Takes facts +
//! pre-derived bits, wires them into the canonical event shape.

use chrono::{DateTime, Utc};
use shiplog::ids::EventId;
use shiplog::schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, SourceRef, SourceSystem,
};

use super::details::PrEventFacts;

pub(super) struct PrEnvelopeInput<'a> {
    pub id: EventId,
    pub occurred_at: DateTime<Utc>,
    pub state: PullRequestState,
    pub actor_login: &'a str,
    pub repo_full_name: String,
    pub repo_html_url: String,
    pub number: u64,
    pub facts: PrEventFacts,
    pub html_url: &'a str,
    pub api_url: &'a str,
    pub source_opaque_id: String,
}

pub(super) fn build_envelope(input: PrEnvelopeInput<'_>) -> EventEnvelope {
    EventEnvelope {
        id: input.id,
        kind: EventKind::PullRequest,
        occurred_at: input.occurred_at,
        actor: Actor {
            login: input.actor_login.to_string(),
            id: None,
        },
        repo: RepoRef {
            full_name: input.repo_full_name,
            html_url: Some(input.repo_html_url),
            visibility: input.facts.visibility,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: input.number,
            title: input.facts.title,
            state: input.state,
            created_at: input.facts.created_at,
            merged_at: input.facts.merged_at,
            additions: input.facts.additions,
            deletions: input.facts.deletions,
            changed_files: input.facts.changed_files,
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: input.html_url.to_string(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(input.api_url.to_string()),
            opaque_id: Some(input.source_opaque_id),
        },
    }
}
