//! Comprehensive tests for shiplog-ingest-linear: parsing, construction, edge cases.

use chrono::NaiveDate;
use shiplog_ingest_linear::{IssueStatus, LinearIngestor};
use std::str::FromStr;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_ingestor() -> LinearIngestor {
    LinearIngestor::new(
        "alice".into(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
    )
}

// ── LinearIngestor construction ─────────────────────────────────────────────

#[test]
fn new_ingestor_has_correct_defaults() {
    let ing = make_ingestor();
    assert_eq!(ing.user, "alice");
    assert_eq!(ing.since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(ing.until, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    assert_eq!(ing.status, IssueStatus::Done);
    assert_eq!(ing.throttle_ms, 0);
    assert!(ing.api_key.is_none());
    assert!(ing.project.is_none());
    assert!(ing.cache.is_none());
}

// ── with_api_key ────────────────────────────────────────────────────────────

#[test]
fn with_api_key_valid() {
    let ing = make_ingestor()
        .with_api_key("lin_api_key_123".into())
        .unwrap();
    assert_eq!(ing.api_key.as_deref(), Some("lin_api_key_123"));
}

#[test]
fn with_api_key_empty_fails() {
    let err = make_ingestor().with_api_key("".into()).unwrap_err();
    assert!(err.to_string().contains("cannot be empty"));
}

#[test]
fn with_api_key_whitespace_accepted() {
    let ing = make_ingestor().with_api_key("  ".into()).unwrap();
    assert_eq!(ing.api_key.as_deref(), Some("  "));
}

// ── with_project ────────────────────────────────────────────────────────────

#[test]
fn with_project_sets_filter() {
    let ing = make_ingestor().with_project("my-project".into());
    assert_eq!(ing.project.as_deref(), Some("my-project"));
}

#[test]
fn with_project_empty_string() {
    let ing = make_ingestor().with_project("".into());
    assert_eq!(ing.project.as_deref(), Some(""));
}

// ── with_status / with_throttle ─────────────────────────────────────────────

#[test]
fn with_status_changes_filter() {
    let ing = make_ingestor().with_status(IssueStatus::Backlog);
    assert_eq!(ing.status, IssueStatus::Backlog);
}

#[test]
fn with_throttle_sets_delay() {
    let ing = make_ingestor().with_throttle(250);
    assert_eq!(ing.throttle_ms, 250);
}

#[test]
fn with_throttle_zero() {
    let ing = make_ingestor().with_throttle(0);
    assert_eq!(ing.throttle_ms, 0);
}

// ── with_cache ──────────────────────────────────────────────────────────────

#[test]
fn with_cache_creates_nested_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let cache_dir = tmp.path().join("x").join("y");
    let ing = make_ingestor().with_cache(&cache_dir).unwrap();
    assert!(ing.cache.is_some());
    assert!(cache_dir.join("linear-api-cache.db").exists());
}

#[test]
fn with_in_memory_cache() {
    let ing = make_ingestor().with_in_memory_cache().unwrap();
    assert!(ing.cache.is_some());
}

// ── IssueStatus ─────────────────────────────────────────────────────────────

#[test]
fn issue_status_as_str_all_variants() {
    assert_eq!(IssueStatus::Backlog.as_str(), "backlog");
    assert_eq!(IssueStatus::Todo.as_str(), "todo");
    assert_eq!(IssueStatus::InProgress.as_str(), "in_progress");
    assert_eq!(IssueStatus::Done.as_str(), "done");
    assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
    assert_eq!(IssueStatus::All.as_str(), "all");
}

#[test]
fn issue_status_from_str_exact() {
    assert_eq!(IssueStatus::from_str("backlog").unwrap(), IssueStatus::Backlog);
    assert_eq!(IssueStatus::from_str("todo").unwrap(), IssueStatus::Todo);
    assert_eq!(
        IssueStatus::from_str("in_progress").unwrap(),
        IssueStatus::InProgress
    );
    assert_eq!(IssueStatus::from_str("done").unwrap(), IssueStatus::Done);
    assert_eq!(
        IssueStatus::from_str("cancelled").unwrap(),
        IssueStatus::Cancelled
    );
    assert_eq!(IssueStatus::from_str("all").unwrap(), IssueStatus::All);
}

#[test]
fn issue_status_from_str_case_insensitive() {
    assert_eq!(IssueStatus::from_str("BACKLOG").unwrap(), IssueStatus::Backlog);
    assert_eq!(IssueStatus::from_str("TODO").unwrap(), IssueStatus::Todo);
    assert_eq!(IssueStatus::from_str("Done").unwrap(), IssueStatus::Done);
}

#[test]
fn issue_status_from_str_aliases() {
    assert_eq!(
        IssueStatus::from_str("in progress").unwrap(),
        IssueStatus::InProgress
    );
    assert_eq!(
        IssueStatus::from_str("completed").unwrap(),
        IssueStatus::Done
    );
    assert_eq!(IssueStatus::from_str("closed").unwrap(), IssueStatus::Done);
    assert_eq!(
        IssueStatus::from_str("canceled").unwrap(),
        IssueStatus::Cancelled
    );
}

#[test]
fn issue_status_from_str_invalid() {
    let err = IssueStatus::from_str("nope").unwrap_err();
    assert!(err.to_string().contains("Invalid issue status"));
    assert!(err.to_string().contains("nope"));
}

#[test]
fn issue_status_from_str_empty() {
    assert!(IssueStatus::from_str("").is_err());
}

#[test]
fn issue_status_eq_and_ne() {
    assert_eq!(IssueStatus::Backlog, IssueStatus::Backlog);
    assert_ne!(IssueStatus::Backlog, IssueStatus::Todo);
    assert_ne!(IssueStatus::Done, IssueStatus::Cancelled);
}

#[test]
fn issue_status_copy() {
    let s = IssueStatus::InProgress;
    let c = s; // Copy
    assert_eq!(c, IssueStatus::InProgress);
    assert_eq!(s, IssueStatus::InProgress); // still valid after copy
}

#[test]
fn issue_status_debug() {
    let debug = format!("{:?}", IssueStatus::Cancelled);
    assert!(debug.contains("Cancelled"));
}

// ── Constant URLs (these are fixed, not dependent on instance) ──────────────

#[test]
fn linear_ingestor_user_stored() {
    let ing = make_ingestor();
    assert_eq!(ing.user, "alice");
}

#[test]
fn linear_ingestor_default_status_is_done() {
    let ing = make_ingestor();
    assert_eq!(ing.status, IssueStatus::Done);
}

// ── Builder chaining ────────────────────────────────────────────────────────

#[test]
fn builder_chain_full() {
    let tmp = tempfile::tempdir().unwrap();
    let ing = LinearIngestor::new(
        "bob".into(),
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    )
    .with_api_key("lin_key".into())
    .unwrap()
    .with_project("proj-1".into())
    .with_status(IssueStatus::Todo)
    .with_throttle(200)
    .with_cache(tmp.path())
    .unwrap();

    assert_eq!(ing.user, "bob");
    assert_eq!(ing.api_key.as_deref(), Some("lin_key"));
    assert_eq!(ing.project.as_deref(), Some("proj-1"));
    assert_eq!(ing.status, IssueStatus::Todo);
    assert_eq!(ing.throttle_ms, 200);
    assert!(ing.cache.is_some());
}

#[test]
fn multiple_with_project_last_wins() {
    let ing = make_ingestor()
        .with_project("first".into())
        .with_project("second".into());
    assert_eq!(ing.project.as_deref(), Some("second"));
}

#[test]
fn multiple_with_status_last_wins() {
    let ing = make_ingestor()
        .with_status(IssueStatus::Backlog)
        .with_status(IssueStatus::Done);
    assert_eq!(ing.status, IssueStatus::Done);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn strategy_issue_status() -> impl Strategy<Value = IssueStatus> {
        prop_oneof![
            Just(IssueStatus::Backlog),
            Just(IssueStatus::Todo),
            Just(IssueStatus::InProgress),
            Just(IssueStatus::Done),
            Just(IssueStatus::Cancelled),
            Just(IssueStatus::All),
        ]
    }

    proptest! {
        #[test]
        fn issue_status_as_str_is_lowercase(status in strategy_issue_status()) {
            let s = status.as_str();
            let lower = s.to_lowercase();
            prop_assert_eq!(s, lower.as_str());
        }

        #[test]
        fn with_throttle_stores_value(ms in 0u64..100_000) {
            let ing = make_ingestor().with_throttle(ms);
            prop_assert_eq!(ing.throttle_ms, ms);
        }

        #[test]
        fn non_empty_api_key_accepted(key in "[a-zA-Z0-9_]{1,100}") {
            let result = make_ingestor().with_api_key(key);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn with_project_stores_value(proj in "[a-z0-9-]{1,50}") {
            let ing = make_ingestor().with_project(proj.clone());
            prop_assert_eq!(ing.project.as_deref(), Some(proj.as_str()));
        }
    }
}
