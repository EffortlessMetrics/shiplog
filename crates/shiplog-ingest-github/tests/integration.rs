//! Integration tests for shiplog-ingest-github public API.
//!
//! These tests exercise the public interface without making real network calls.

use chrono::NaiveDate;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ports::Ingestor;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ── Constructor & builder tests ─────────────────────────────────────────

#[test]
fn new_sets_user_and_dates() {
    let ing = GithubIngestor::new("alice".into(), date(2025, 1, 1), date(2025, 3, 1));
    assert_eq!(ing.user, "alice");
    assert_eq!(ing.since, date(2025, 1, 1));
    assert_eq!(ing.until, date(2025, 3, 1));
}

#[test]
fn default_mode_is_merged() {
    let ing = GithubIngestor::new("bob".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.mode, "merged");
}

#[test]
fn default_api_base_is_github_com() {
    let ing = GithubIngestor::new("carol".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.api_base, "https://api.github.com");
}

#[test]
fn include_reviews_defaults_to_false() {
    let ing = GithubIngestor::new("dave".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(!ing.include_reviews);
}

#[test]
fn fetch_details_defaults_to_true() {
    let ing = GithubIngestor::new("eve".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.fetch_details);
}

#[test]
fn throttle_ms_defaults_to_zero() {
    let ing = GithubIngestor::new("frank".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.throttle_ms, 0);
}

#[test]
fn token_defaults_to_none() {
    let ing = GithubIngestor::new("grace".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.token.is_none());
}

#[test]
fn cache_defaults_to_none() {
    let ing = GithubIngestor::new("heidi".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(ing.cache.is_none());
}

// ── Builder modifications ───────────────────────────────────────────────

#[test]
fn fields_can_be_modified_directly() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 6, 1));
    ing.mode = "created".to_string();
    ing.include_reviews = true;
    ing.fetch_details = false;
    ing.throttle_ms = 500;
    ing.token = Some("ghp_test123".to_string());
    ing.api_base = "https://ghes.corp/api/v3".to_string();

    assert_eq!(ing.mode, "created");
    assert!(ing.include_reviews);
    assert!(!ing.fetch_details);
    assert_eq!(ing.throttle_ms, 500);
    assert_eq!(ing.token.as_deref(), Some("ghp_test123"));
    assert_eq!(ing.api_base, "https://ghes.corp/api/v3");
}

// ── Cache integration ───────────────────────────────────────────────────

#[test]
fn with_cache_creates_db_file() {
    let temp = tempfile::tempdir().unwrap();
    let cache_dir = temp.path().join("deep").join("nested");

    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache(&cache_dir)
        .unwrap();

    assert!(ing.cache.is_some());
    assert!(cache_dir.join("github-api-cache.db").exists());
}

#[test]
fn with_in_memory_cache_succeeds() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_in_memory_cache()
        .unwrap();
    assert!(ing.cache.is_some());
}

#[test]
fn cache_can_be_replaced() {
    let temp = tempfile::tempdir().unwrap();
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache(temp.path())
        .unwrap()
        .with_in_memory_cache()
        .unwrap();
    assert!(ing.cache.is_some());
}

// ── Ingestor trait: error conditions (no network) ───────────────────────

#[test]
fn ingest_errors_when_since_equals_until() {
    let d = date(2025, 6, 1);
    let ing = GithubIngestor::new("user".into(), d, d);
    let err = ing.ingest().unwrap_err();
    assert!(
        err.to_string().contains("since must be < until"),
        "unexpected error: {err}"
    );
}

#[test]
fn ingest_errors_when_since_after_until() {
    let ing = GithubIngestor::new("user".into(), date(2025, 12, 1), date(2025, 1, 1));
    let err = ing.ingest().unwrap_err();
    assert!(
        err.to_string().contains("since must be < until"),
        "unexpected error: {err}"
    );
}
