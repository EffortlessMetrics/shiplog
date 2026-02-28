//! Boundary value tests for shiplog-coverage.
//!
//! Covers 0%/100% coverage, single-event completeness, and very large event counts.

use chrono::NaiveDate;
use shiplog_coverage::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_schema::coverage::{Completeness, CoverageSlice, TimeWindow};
use shiplog_testkit::bdd::builders::CoverageBuilder;

// ============================================================================
// 0% coverage boundary
// ============================================================================

#[test]
fn zero_percent_coverage_fetched_zero_of_many() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 100,
        fetched: 0,
        incomplete_results: Some(true),
        notes: vec!["Completely rate limited".to_string()],
    };
    assert_eq!(slice.fetched, 0);
    assert!(slice.fetched < slice.total_count);
}

#[test]
fn zero_events_total_is_complete() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 0,
        fetched: 0,
        incomplete_results: Some(false),
        notes: vec![],
    };
    assert_eq!(slice.fetched, slice.total_count);
    assert_eq!(slice.total_count, 0);
    assert_eq!(slice.incomplete_results, Some(false));
}

#[test]
fn manifest_with_zero_events_is_complete() {
    let manifest = CoverageBuilder::new("empty-user")
        .completeness(Completeness::Complete)
        .build();
    assert!(manifest.slices.is_empty());
    assert_eq!(manifest.completeness, Completeness::Complete);
}

// ============================================================================
// 100% coverage boundary
// ============================================================================

#[test]
fn full_coverage_all_slices_complete() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    let slices: Vec<CoverageSlice> = windows
        .iter()
        .map(|w| CoverageSlice {
            window: w.clone(),
            query: "github prs".to_string(),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec![],
        })
        .collect();

    assert!(slices.iter().all(|s| s.fetched == s.total_count));
    assert!(slices.iter().all(|s| s.incomplete_results == Some(false)));
    assert!(slices.iter().all(|s| s.notes.is_empty()));
}

#[test]
fn manifest_complete_with_full_date_range() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
    let manifest = CoverageBuilder::new("full-user")
        .dates(since, until)
        .completeness(Completeness::Complete)
        .build();
    assert_eq!(manifest.completeness, Completeness::Complete);
    assert_eq!(manifest.window.since, since);
    assert_eq!(manifest.window.until, until);
}

// ============================================================================
// Single event = 100% when all receipts present
// ============================================================================

#[test]
fn single_event_single_slice_all_fetched() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 3, 2).unwrap(),
    };
    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 1,
        fetched: 1,
        incomplete_results: Some(false),
        notes: vec![],
    };
    assert_eq!(slice.fetched, 1);
    assert_eq!(slice.total_count, 1);
    assert_eq!(slice.fetched, slice.total_count);
}

#[test]
fn single_day_window_coverage_is_one_day() {
    let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 16).unwrap();
    let windows = day_windows(since, until);
    assert_eq!(windows.len(), 1);
    assert_eq!(window_len_days(&windows[0]), 1);
}

// ============================================================================
// Very large event counts
// ============================================================================

#[test]
fn very_large_total_count_all_fetched() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 1_000_000,
        fetched: 1_000_000,
        incomplete_results: Some(false),
        notes: vec![],
    };
    assert_eq!(slice.fetched, slice.total_count);
}

#[test]
fn very_large_total_with_partial_fetch() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };
    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: u64::MAX,
        fetched: 1000,
        incomplete_results: Some(true),
        notes: vec!["Extremely large result set".to_string()],
    };
    assert!(slice.fetched < slice.total_count);
    assert_eq!(slice.incomplete_results, Some(true));
}

#[test]
fn many_slices_all_complete() {
    let since = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let windows = month_windows(since, until);
    assert_eq!(windows.len(), 120);

    let slices: Vec<CoverageSlice> = windows
        .iter()
        .enumerate()
        .map(|(i, w)| CoverageSlice {
            window: w.clone(),
            query: format!("github prs month {i}"),
            total_count: 50,
            fetched: 50,
            incomplete_results: Some(false),
            notes: vec![],
        })
        .collect();

    assert_eq!(slices.len(), 120);
    let total_fetched: u64 = slices.iter().map(|s| s.fetched).sum();
    assert_eq!(total_fetched, 120 * 50);
    assert!(slices.iter().all(|s| s.fetched == s.total_count));
}

#[test]
fn many_warnings_on_manifest() {
    let mut manifest = CoverageBuilder::new("warn-user")
        .completeness(Completeness::Partial)
        .build();

    for i in 0..1000 {
        manifest
            .warnings
            .push(format!("Missing PR #{i} from acme/repo"));
    }
    assert_eq!(manifest.warnings.len(), 1000);
}

// ============================================================================
// Edge: week windows across leap day
// ============================================================================

#[test]
fn week_windows_spanning_leap_day() {
    let since = NaiveDate::from_ymd_opt(2024, 2, 26).unwrap(); // Monday
    let until = NaiveDate::from_ymd_opt(2024, 3, 4).unwrap(); // Monday
    let w = week_windows(since, until);
    assert_eq!(w.len(), 1);
    assert_eq!(window_len_days(&w[0]), 7); // Includes Feb 29
}

#[test]
fn day_windows_full_feb_leap_year() {
    let since = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 29);
}

#[test]
fn day_windows_full_feb_non_leap_year() {
    let since = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 28);
}
