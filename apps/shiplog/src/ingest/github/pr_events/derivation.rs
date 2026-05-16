//! Pure derivations from PR facts: `occurred_at`, state, and the
//! deterministic event id. No I/O, no `GithubIngestor` access — these
//! are the trivially testable seams that fall out of the SRP split.

use chrono::{DateTime, Utc};
use shiplog::ids::EventId;
use shiplog::schema::event::PullRequestState;

/// Pick the timestamp the PR is recorded at: `merged_at` (falling back to
/// `created_at`) in `merged` mode; `created_at` in `created` mode.
pub(super) fn occurred_at_for_mode(
    mode: &str,
    created_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
) -> DateTime<Utc> {
    match mode {
        "created" => created_at,
        _ => merged_at.unwrap_or(created_at),
    }
}

/// `Merged` when a merge timestamp is present, `Unknown` otherwise. We
/// don't synthesise `Open`/`Closed` because the search row doesn't carry
/// enough information to tell them apart without a detail fetch.
pub(super) fn state_from_merged_at(merged_at: Option<DateTime<Utc>>) -> PullRequestState {
    if merged_at.is_some() {
        PullRequestState::Merged
    } else {
        PullRequestState::Unknown
    }
}

/// Deterministic ID for a GitHub PR event.
pub(super) fn pr_event_id(repo_full_name: &str, number: u64) -> EventId {
    EventId::from_parts(["github", "pr", repo_full_name, &number.to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    const CREATED: DateTime<Utc> = DateTime::<Utc>::UNIX_EPOCH;

    #[test]
    fn merged_mode_prefers_merged_then_falls_back_to_created() {
        let merged = CREATED + Duration::days(5);
        assert_eq!(
            occurred_at_for_mode("merged", CREATED, Some(merged)),
            merged
        );
        assert_eq!(occurred_at_for_mode("merged", CREATED, None), CREATED);
    }

    #[test]
    fn created_mode_always_uses_created() {
        let merged = CREATED + Duration::days(5);
        assert_eq!(
            occurred_at_for_mode("created", CREATED, Some(merged)),
            CREATED
        );
    }

    #[test]
    fn state_reflects_merge_presence() {
        assert_eq!(
            state_from_merged_at(Some(CREATED)),
            PullRequestState::Merged
        );
        assert_eq!(state_from_merged_at(None), PullRequestState::Unknown);
    }

    #[test]
    fn pr_event_id_is_deterministic() {
        let a = pr_event_id("org/repo", 42);
        let b = pr_event_id("org/repo", 42);
        let c = pr_event_id("org/repo", 43);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
