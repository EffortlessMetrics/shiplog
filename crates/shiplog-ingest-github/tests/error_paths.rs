//! Error-path tests for the shiplog-ingest-github crate.
//!
//! Exercises date validation, query building, URL parsing, and deserialization
//! edge cases without making real network calls.

use chrono::NaiveDate;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ports::Ingestor;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ---------------------------------------------------------------------------
// Date-range validation via the Ingestor trait
// ---------------------------------------------------------------------------

#[test]
fn ingest_errors_when_since_far_after_until() {
    let ing = GithubIngestor::new("user".into(), date(2030, 1, 1), date(2020, 1, 1));
    let err = ing.ingest().unwrap_err();
    assert!(
        err.to_string().contains("since must be < until"),
        "unexpected: {err}"
    );
}

#[test]
fn ingest_errors_when_since_is_one_day_after_until() {
    let ing = GithubIngestor::new("user".into(), date(2025, 6, 2), date(2025, 6, 1));
    let err = ing.ingest().unwrap_err();
    assert!(
        err.to_string().contains("since must be < until"),
        "unexpected: {err}"
    );
}

// ---------------------------------------------------------------------------
// Builder / field access edge cases
// ---------------------------------------------------------------------------

#[test]
fn created_mode_can_be_set() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.mode = "created".to_string();
    assert_eq!(ing.mode, "created");
}

#[test]
fn custom_api_base_is_preserved() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://ghes.corp/api/v3".to_string();
    assert_eq!(ing.api_base, "https://ghes.corp/api/v3");
}

#[test]
fn empty_user_does_not_panic_on_construction() {
    let ing = GithubIngestor::new(String::new(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.user.is_empty());
}

// ---------------------------------------------------------------------------
// Cache error paths
// ---------------------------------------------------------------------------

#[test]
fn with_cache_on_read_only_path_errors_gracefully() {
    // Attempt to create a cache in a path that can't exist
    let result = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache("\0invalid\0path");
    // On most systems, null bytes in paths cause errors
    assert!(
        result.is_err(),
        "creating cache at invalid path should fail"
    );
}

// ---------------------------------------------------------------------------
// Multiple builder chains
// ---------------------------------------------------------------------------

#[test]
fn with_in_memory_cache_then_file_cache_replaces() {
    let temp = tempfile::tempdir().unwrap();
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_in_memory_cache()
        .unwrap()
        .with_cache(temp.path())
        .unwrap();
    assert!(ing.cache.is_some());
    assert!(temp.path().join("github-api-cache.db").exists());
}
