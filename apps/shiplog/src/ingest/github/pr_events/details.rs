//! Resolve the per-PR facts an event envelope needs, from either a fetched
//! `PullRequestDetails` payload or — when detail fetching is disabled or
//! fails — the bare fields already present on the search result.
//!
//! Both helpers are pure: they touch no network and no `GithubIngestor`
//! state, so the fetch-vs-fallback branch in [`super::items_to_pr_events`]
//! becomes a straightforward dispatch.

use chrono::{DateTime, Utc};
use shiplog::schema::event::RepoVisibility;

use super::super::{PullRequestDetails, SearchIssueItem};

/// The subset of PR data needed to build an [`EventEnvelope`].
///
/// [`EventEnvelope`]: shiplog::schema::event::EventEnvelope
pub(super) struct PrEventFacts {
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub additions: Option<u64>,
    pub deletions: Option<u64>,
    pub changed_files: Option<u64>,
    pub visibility: RepoVisibility,
}

/// Build facts from a successful detail fetch. Visibility comes from
/// the `base.repo.private` flag; the diff stats and merge timestamp
/// are only available from this path.
pub(super) fn facts_from_details(d: &PullRequestDetails) -> PrEventFacts {
    let visibility = if d.base.repo.private_field {
        RepoVisibility::Private
    } else {
        RepoVisibility::Public
    };
    PrEventFacts {
        title: d.title.clone(),
        created_at: d.created_at,
        merged_at: d.merged_at,
        additions: Some(d.additions),
        deletions: Some(d.deletions),
        changed_files: Some(d.changed_files),
        visibility,
    }
}

/// Build facts from the bare search-item fields. Used when
/// `fetch_details=false` or when a detail fetch errors out — the
/// caller's contract is to fall back rather than fail the whole batch.
pub(super) fn facts_from_search_item(item: &SearchIssueItem) -> PrEventFacts {
    PrEventFacts {
        title: item.title.clone(),
        created_at: item.created_at.unwrap_or_else(Utc::now),
        merged_at: None,
        additions: None,
        deletions: None,
        changed_files: None,
        visibility: RepoVisibility::Unknown,
    }
}
