//! Comprehensive tests for shiplog-ingest-jira: parsing, construction, edge cases.

use chrono::NaiveDate;
use shiplog_ingest_jira::{IssueStatus, JiraIngestor};
use std::str::FromStr;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_ingestor() -> JiraIngestor {
    JiraIngestor::new(
        "alice".into(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
    )
}

// ── JiraIngestor construction ───────────────────────────────────────────────

#[test]
fn new_ingestor_has_correct_defaults() {
    let ing = make_ingestor();
    assert_eq!(ing.user, "alice");
    assert_eq!(ing.since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(ing.until, NaiveDate::from_ymd_opt(2025, 6, 1).unwrap());
    assert_eq!(ing.status, IssueStatus::Done);
    assert_eq!(ing.throttle_ms, 0);
    assert!(ing.token.is_none());
    assert_eq!(ing.instance, "jira.atlassian.com");
    assert!(ing.cache.is_none());
}

// ── with_token ──────────────────────────────────────────────────────────────

#[test]
fn with_token_valid() {
    let ing = make_ingestor().with_token("my-token".into()).unwrap();
    assert_eq!(ing.token.as_deref(), Some("my-token"));
}

#[test]
fn with_token_empty_fails() {
    let err = make_ingestor().with_token("".into()).unwrap_err();
    assert!(err.to_string().contains("cannot be empty"));
}

#[test]
fn with_token_whitespace_only_is_accepted() {
    // The API only checks for empty, not whitespace-only
    let ing = make_ingestor().with_token("  ".into()).unwrap();
    assert_eq!(ing.token.as_deref(), Some("  "));
}

// ── with_instance ───────────────────────────────────────────────────────────

#[test]
fn with_instance_plain_hostname() {
    let ing = make_ingestor()
        .with_instance("mycompany.atlassian.net".into())
        .unwrap();
    assert_eq!(ing.instance, "mycompany.atlassian.net");
}

#[test]
fn with_instance_strips_https() {
    let ing = make_ingestor()
        .with_instance("https://company.atlassian.net".into())
        .unwrap();
    assert_eq!(ing.instance, "company.atlassian.net");
}

#[test]
fn with_instance_strips_http() {
    let ing = make_ingestor()
        .with_instance("http://jira.internal.corp".into())
        .unwrap();
    assert_eq!(ing.instance, "jira.internal.corp");
}

#[test]
fn with_instance_empty_fails() {
    let err = make_ingestor().with_instance("".into()).unwrap_err();
    assert!(err.to_string().contains("cannot be empty"));
}

#[test]
fn with_instance_protocol_only_fails() {
    let err = make_ingestor().with_instance("http://".into()).unwrap_err();
    assert!(err.to_string().contains("Invalid"));
}

#[test]
fn with_instance_strips_trailing_path_from_url() {
    let ing = make_ingestor()
        .with_instance("https://jira.example.com/some/path".into())
        .unwrap();
    assert_eq!(ing.instance, "jira.example.com");
}

// ── with_status / with_throttle ─────────────────────────────────────────────

#[test]
fn with_status_changes_filter() {
    let ing = make_ingestor().with_status(IssueStatus::All);
    assert_eq!(ing.status, IssueStatus::All);
}

#[test]
fn with_throttle_sets_delay() {
    let ing = make_ingestor().with_throttle(500);
    assert_eq!(ing.throttle_ms, 500);
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
    let cache_dir = tmp.path().join("a").join("b").join("c");
    let ing = make_ingestor().with_cache(&cache_dir).unwrap();
    assert!(ing.cache.is_some());
    assert!(cache_dir.join("jira-api-cache.db").exists());
}

#[test]
fn with_in_memory_cache() {
    let ing = make_ingestor().with_in_memory_cache().unwrap();
    assert!(ing.cache.is_some());
}

// ── IssueStatus ─────────────────────────────────────────────────────────────

#[test]
fn issue_status_as_str_roundtrip_open() {
    let status = IssueStatus::Open;
    assert_eq!(status.as_str(), "open");
    assert_eq!(IssueStatus::from_str("open").unwrap(), IssueStatus::Open);
}

#[test]
fn issue_status_as_str_roundtrip_in_progress() {
    let status = IssueStatus::InProgress;
    assert_eq!(status.as_str(), "in_progress");
    assert_eq!(
        IssueStatus::from_str("in_progress").unwrap(),
        IssueStatus::InProgress
    );
}

#[test]
fn issue_status_as_str_roundtrip_done() {
    let status = IssueStatus::Done;
    assert_eq!(status.as_str(), "done");
    assert_eq!(IssueStatus::from_str("done").unwrap(), IssueStatus::Done);
}

#[test]
fn issue_status_as_str_closed() {
    assert_eq!(IssueStatus::Closed.as_str(), "closed");
}

#[test]
fn issue_status_as_str_all() {
    assert_eq!(IssueStatus::All.as_str(), "all");
    assert_eq!(IssueStatus::from_str("all").unwrap(), IssueStatus::All);
}

#[test]
fn issue_status_from_str_case_insensitive() {
    assert_eq!(IssueStatus::from_str("OPEN").unwrap(), IssueStatus::Open);
    assert_eq!(IssueStatus::from_str("Done").unwrap(), IssueStatus::Done);
    assert_eq!(IssueStatus::from_str("ALL").unwrap(), IssueStatus::All);
}

#[test]
fn issue_status_from_str_aliases() {
    assert_eq!(IssueStatus::from_str("to do").unwrap(), IssueStatus::Open);
    assert_eq!(
        IssueStatus::from_str("resolved").unwrap(),
        IssueStatus::Done
    );
    assert_eq!(IssueStatus::from_str("closed").unwrap(), IssueStatus::Done);
}

#[test]
fn issue_status_from_str_invalid() {
    let err = IssueStatus::from_str("unknown_status").unwrap_err();
    assert!(err.to_string().contains("Invalid issue status"));
    assert!(err.to_string().contains("unknown_status"));
}

#[test]
fn issue_status_from_str_empty() {
    let err = IssueStatus::from_str("").unwrap_err();
    assert!(err.to_string().contains("Invalid issue status"));
}

#[test]
fn issue_status_eq() {
    assert_eq!(IssueStatus::Open, IssueStatus::Open);
    assert_ne!(IssueStatus::Open, IssueStatus::Done);
}

#[test]
fn issue_status_clone() {
    let status = IssueStatus::InProgress;
    let cloned = status;
    assert_eq!(cloned, IssueStatus::InProgress);
}

#[test]
fn issue_status_debug() {
    let debug = format!("{:?}", IssueStatus::Done);
    assert!(debug.contains("Done"));
}

// ── Instance field ───────────────────────────────────────────────────────────

#[test]
fn default_instance_is_jira_atlassian() {
    let ing = make_ingestor();
    assert_eq!(ing.instance, "jira.atlassian.com");
}

#[test]
fn custom_instance_is_stored() {
    let mut ing = make_ingestor();
    ing.instance = "myco.atlassian.net".into();
    assert_eq!(ing.instance, "myco.atlassian.net");
}

// ── Builder chaining ────────────────────────────────────────────────────────

#[test]
fn builder_chain_full() {
    let tmp = tempfile::tempdir().unwrap();
    let ing = JiraIngestor::new(
        "bob".into(),
        NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    )
    .with_token("tok123".into())
    .unwrap()
    .with_instance("https://bob.atlassian.net".into())
    .unwrap()
    .with_status(IssueStatus::Open)
    .with_throttle(100)
    .with_cache(tmp.path())
    .unwrap();

    assert_eq!(ing.user, "bob");
    assert_eq!(ing.token.as_deref(), Some("tok123"));
    assert_eq!(ing.instance, "bob.atlassian.net");
    assert_eq!(ing.status, IssueStatus::Open);
    assert_eq!(ing.throttle_ms, 100);
    assert!(ing.cache.is_some());
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn strategy_issue_status() -> impl Strategy<Value = IssueStatus> {
        prop_oneof![
            Just(IssueStatus::Open),
            Just(IssueStatus::InProgress),
            Just(IssueStatus::Done),
            Just(IssueStatus::Closed),
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
        fn with_throttle_is_idempotent(ms in 0u64..10000) {
            let ing = make_ingestor().with_throttle(ms);
            prop_assert_eq!(ing.throttle_ms, ms);
        }

        #[test]
        fn non_empty_token_accepted(token in "[a-zA-Z0-9]{1,100}") {
            let result = make_ingestor().with_token(token);
            prop_assert!(result.is_ok());
        }

        #[test]
        fn plain_hostname_passes_through(host in "[a-z]{3,10}\\.[a-z]{2,5}\\.[a-z]{2,3}") {
            let result = make_ingestor().with_instance(host.clone());
            prop_assert!(result.is_ok());
            prop_assert_eq!(result.unwrap().instance, host);
        }
    }
}
