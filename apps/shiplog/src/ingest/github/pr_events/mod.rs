//! SRP submodules behind `items_to_pr_events`.
//!
//! The previous monolithic ~115-line method bundled four concerns:
//! detail acquisition with fallback, metadata derivation, envelope
//! assembly, and iteration. They split here as:
//!
//! - [`details`]: resolves [`details::PrEventFacts`] from either a
//!   fetched [`super::PullRequestDetails`] or the bare search-item.
//! - [`derivation`]: pure derivations (`occurred_at`, state, [`EventId`]).
//! - [`envelope`]: pure assembler for the canonical [`EventEnvelope`].
//!
//! Only [`items_to_pr_events`] touches the network — via
//! [`super::GithubIngestor::fetch_pr_details`] — keeping the other
//! submodules trivially testable.
//!
//! [`EventEnvelope`]: shiplog::schema::event::EventEnvelope
//! [`EventId`]: shiplog::ids::EventId

mod derivation;
mod details;
mod envelope;

use anyhow::Result;
use reqwest::blocking::Client;
use shiplog::schema::event::EventEnvelope;

use super::{GithubIngestor, SearchIssueItem, repo_from_repo_url};

pub(super) fn items_to_pr_events(
    ing: &GithubIngestor,
    client: &Client,
    items: Vec<SearchIssueItem>,
) -> Result<Vec<EventEnvelope>> {
    let html_base = ing.html_base_url();
    let mut out = Vec::with_capacity(items.len());

    for item in items {
        let Some(pr_ref) = item.pull_request.as_ref() else {
            continue;
        };

        let facts = if ing.fetch_details {
            match ing.fetch_pr_details(client, &pr_ref.url) {
                Ok(d) => details::facts_from_details(&d),
                Err(_) => details::facts_from_search_item(&item),
            }
        } else {
            details::facts_from_search_item(&item)
        };

        let (repo_full_name, repo_html_url) = repo_from_repo_url(&item.repository_url, &html_base);

        let id = derivation::pr_event_id(&repo_full_name, item.number);
        let occurred_at =
            derivation::occurred_at_for_mode(ing.mode.as_str(), facts.created_at, facts.merged_at);
        let state = derivation::state_from_merged_at(facts.merged_at);

        out.push(envelope::build_envelope(envelope::PrEnvelopeInput {
            id,
            occurred_at,
            state,
            actor_login: &ing.user,
            repo_full_name,
            repo_html_url,
            number: item.number,
            facts,
            html_url: &item.html_url,
            api_url: &pr_ref.url,
            source_opaque_id: item.id.to_string(),
        }));
    }

    Ok(out)
}
