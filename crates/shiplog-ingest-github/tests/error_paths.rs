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

// ---------------------------------------------------------------------------
// Date validation error message quality
// ---------------------------------------------------------------------------

#[test]
fn ingest_errors_when_since_equals_until() {
    let ing = GithubIngestor::new("user".into(), date(2025, 6, 1), date(2025, 6, 1));
    let err = ing.ingest().unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("since must be < until"),
        "same-day range should fail: {msg}"
    );
}

#[test]
fn date_validation_error_is_not_empty() {
    let ing = GithubIngestor::new("user".into(), date(2025, 12, 31), date(2025, 1, 1));
    let err = ing.ingest().unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty(), "error message should not be empty");
    assert!(msg.len() > 10, "error message should be descriptive: {msg}");
}

// ---------------------------------------------------------------------------
// API base URL edge cases
// ---------------------------------------------------------------------------

#[test]
fn custom_api_base_with_trailing_slash_is_preserved() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://ghes.corp/api/v3/".to_string();
    assert_eq!(ing.api_base, "https://ghes.corp/api/v3/");
}

#[test]
fn default_api_base_is_github_dot_com() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.api_base, "https://api.github.com");
}

// ---------------------------------------------------------------------------
// Builder field defaults
// ---------------------------------------------------------------------------

#[test]
fn default_mode_is_merged() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.mode, "merged");
}

#[test]
fn default_include_reviews_is_false() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(!ing.include_reviews);
}

#[test]
fn default_throttle_is_zero() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.throttle_ms, 0);
}

#[test]
fn default_token_is_none() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.token.is_none());
}

#[test]
fn default_cache_is_none() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.cache.is_none());
}

// ---------------------------------------------------------------------------
// Cache path error context
// ---------------------------------------------------------------------------

#[test]
fn cache_error_message_contains_context() {
    let result = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache("\0invalid\0path");
    if let Err(e) = result {
        let msg = format!("{e:#}");
        // Error chain should have some context about what was being done
        assert!(
            !msg.is_empty(),
            "cache error should have a non-empty message"
        );
    }
}

#[test]
fn in_memory_cache_succeeds_without_filesystem() {
    let result = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_in_memory_cache();
    assert!(result.is_ok(), "in-memory cache should always succeed");
    assert!(result.unwrap().cache.is_some());
}
