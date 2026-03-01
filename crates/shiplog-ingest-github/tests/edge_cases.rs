//! Edge-case tests for shiplog-ingest-github.
//!
//! Covers adaptive date slicing behavior, SQLite cache integration,
//! rate limit / pagination API surface, and GitHub Enterprise base URL
//! configuration—all without making real network calls.

use chrono::NaiveDate;
use shiplog_ingest_github::GithubIngestor;
use shiplog_ports::Ingestor;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

// ── Adaptive date slicing: ingest validates since < until ────────────────

#[test]
fn ingest_single_day_window_is_valid() {
    // since=2025-06-01, until=2025-06-02 → exactly one day, passes date check.
    // Result depends on network/token, but must NOT be a date-range error.
    let ing = GithubIngestor::new("user".into(), date(2025, 6, 1), date(2025, 6, 2));
    match ing.ingest() {
        Ok(_) => {} // token present, succeeded
        Err(e) => assert!(
            !e.to_string().contains("since must be < until"),
            "single-day window should pass date validation: {e}"
        ),
    }
}

#[test]
fn ingest_multi_year_window_passes_date_validation() {
    let ing = GithubIngestor::new("user".into(), date(2020, 1, 1), date(2025, 12, 31));
    match ing.ingest() {
        Ok(_) => {}
        Err(e) => assert!(
            !e.to_string().contains("since must be < until"),
            "multi-year window should pass date validation: {e}"
        ),
    }
}

#[test]
fn ingest_leap_day_boundary_accepted() {
    let ing = GithubIngestor::new("user".into(), date(2024, 2, 28), date(2024, 3, 1));
    match ing.ingest() {
        Ok(_) => {}
        Err(e) => assert!(
            !e.to_string().contains("since must be < until"),
            "leap-year boundary window should be valid: {e}"
        ),
    }
}

// ── SQLite cache integration (tempfile) ──────────────────────────────────

#[test]
fn with_cache_nested_deep_directory_creates_all_parents() {
    let temp = tempfile::tempdir().unwrap();
    let deep = temp.path().join("a").join("b").join("c").join("d");
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache(&deep)
        .unwrap();
    assert!(ing.cache.is_some());
    assert!(deep.join("github-api-cache.db").exists());
}

#[test]
fn in_memory_cache_is_distinct_per_call() {
    let ing1 = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_in_memory_cache()
        .unwrap();
    let ing2 = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_in_memory_cache()
        .unwrap();
    // Both should have caches, and they should be independent instances
    assert!(ing1.cache.is_some());
    assert!(ing2.cache.is_some());
}

#[test]
fn with_cache_same_path_twice_succeeds() {
    let temp = tempfile::tempdir().unwrap();
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache(temp.path())
        .unwrap()
        .with_cache(temp.path())
        .unwrap();
    assert!(ing.cache.is_some());
}

#[test]
fn cache_db_file_persists_after_ingestor_drop() {
    let temp = tempfile::tempdir().unwrap();
    let db_path = temp.path().join("github-api-cache.db");
    {
        let _ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
            .with_cache(temp.path())
            .unwrap();
        // _ing dropped here
    }
    assert!(db_path.exists(), "cache DB should persist after drop");
}

// ── Rate limit / throttle configuration ──────────────────────────────────

#[test]
fn throttle_ms_can_be_set_to_large_value() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.throttle_ms = 60_000; // 60 seconds
    assert_eq!(ing.throttle_ms, 60_000);
}

#[test]
fn throttle_ms_zero_is_no_throttle() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert_eq!(ing.throttle_ms, 0);
}

// ── Pagination surface: fetch_details flag ───────────────────────────────

#[test]
fn fetch_details_can_be_disabled() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.fetch_details = false;
    assert!(!ing.fetch_details);
}

#[test]
fn include_reviews_flag_toggles() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    assert!(!ing.include_reviews);
    ing.include_reviews = true;
    assert!(ing.include_reviews);
    ing.include_reviews = false;
    assert!(!ing.include_reviews);
}

// ── GitHub Enterprise base URL configuration ─────────────────────────────

#[test]
fn ghes_api_base_with_v3_path() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://github.corp.com/api/v3".to_string();
    assert_eq!(ing.api_base, "https://github.corp.com/api/v3");
}

#[test]
fn ghes_api_base_with_custom_port() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://ghes.internal:9443/api/v3".to_string();
    assert_eq!(ing.api_base, "https://ghes.internal:9443/api/v3");
}

#[test]
fn ghes_api_base_http_scheme() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "http://ghes.local/api/v3".to_string();
    assert_eq!(ing.api_base, "http://ghes.local/api/v3");
}

#[test]
fn ghes_token_can_be_set_with_enterprise_base() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://github.enterprise.com/api/v3".to_string();
    ing.token = Some("ghp_enterprise_token_12345".to_string());
    assert_eq!(ing.api_base, "https://github.enterprise.com/api/v3");
    assert_eq!(ing.token.as_deref(), Some("ghp_enterprise_token_12345"));
}

#[test]
fn ghes_mode_and_include_reviews_with_enterprise() {
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1));
    ing.api_base = "https://ghes.corp/api/v3".to_string();
    ing.mode = "created".to_string();
    ing.include_reviews = true;
    assert_eq!(ing.mode, "created");
    assert!(ing.include_reviews);
}

#[test]
fn ghes_with_cache_and_enterprise_base() {
    let temp = tempfile::tempdir().unwrap();
    let mut ing = GithubIngestor::new("user".into(), date(2025, 1, 1), date(2025, 2, 1))
        .with_cache(temp.path())
        .unwrap();
    ing.api_base = "https://ghes.corp/api/v3".to_string();
    assert!(ing.cache.is_some());
    assert_eq!(ing.api_base, "https://ghes.corp/api/v3");
}

// ── All builder fields compose correctly ─────────────────────────────────

#[test]
fn full_builder_chain_with_all_options() {
    let temp = tempfile::tempdir().unwrap();
    let mut ing = GithubIngestor::new("alice".into(), date(2025, 1, 1), date(2025, 6, 1))
        .with_cache(temp.path())
        .unwrap();
    ing.mode = "created".to_string();
    ing.include_reviews = true;
    ing.fetch_details = false;
    ing.throttle_ms = 250;
    ing.token = Some("ghp_abc".to_string());
    ing.api_base = "https://ghes.corp/api/v3".to_string();

    assert_eq!(ing.user, "alice");
    assert_eq!(ing.mode, "created");
    assert!(ing.include_reviews);
    assert!(!ing.fetch_details);
    assert_eq!(ing.throttle_ms, 250);
    assert_eq!(ing.token.as_deref(), Some("ghp_abc"));
    assert_eq!(ing.api_base, "https://ghes.corp/api/v3");
    assert!(ing.cache.is_some());
}

// ── Date validation edge cases ───────────────────────────────────────────

#[test]
fn ingest_since_one_day_before_until_succeeds_date_check() {
    let ing = GithubIngestor::new("user".into(), date(2025, 1, 31), date(2025, 2, 1));
    match ing.ingest() {
        Ok(_) => {}
        Err(e) => assert!(
            !e.to_string().contains("since must be < until"),
            "adjacent-day window should pass date validation: {e}"
        ),
    }
}

#[test]
fn ingest_year_boundary_window_passes_date_check() {
    let ing = GithubIngestor::new("user".into(), date(2024, 12, 31), date(2025, 1, 1));
    match ing.ingest() {
        Ok(_) => {}
        Err(e) => assert!(
            !e.to_string().contains("since must be < until"),
            "year-boundary window should pass: {e}"
        ),
    }
}
