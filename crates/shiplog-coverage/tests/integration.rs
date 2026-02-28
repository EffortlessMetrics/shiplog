//! Integration tests for shiplog-coverage
//!
//! Tests cover coverage manifest construction, completeness checking,
//! and date-range slicing across the public API surface.

use anyhow::Result;
use chrono::NaiveDate;
use shiplog_coverage::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_schema::coverage::{
    Completeness, CoverageSlice, TimeWindow,
};
use shiplog_testkit::bdd::builders::CoverageBuilder;

// ============================================================================
// Coverage manifest generation with various event combinations
// ============================================================================

#[test]
fn manifest_with_no_slices_defaults_to_complete() -> Result<()> {
    let manifest = CoverageBuilder::new("alice")
        .completeness(Completeness::Complete)
        .build();

    assert!(manifest.slices.is_empty());
    assert_eq!(manifest.completeness, Completeness::Complete);
    assert_eq!(manifest.user, "alice");
    Ok(())
}

#[test]
fn manifest_with_partial_completeness_reflects_missing_data() -> Result<()> {
    let manifest = CoverageBuilder::new("bob")
        .completeness(Completeness::Partial)
        .build();

    assert_eq!(manifest.completeness, Completeness::Partial);
    Ok(())
}

#[test]
fn manifest_with_unknown_completeness() -> Result<()> {
    let manifest = CoverageBuilder::new("carol")
        .completeness(Completeness::Unknown)
        .build();

    assert_eq!(manifest.completeness, Completeness::Unknown);
    Ok(())
}

#[test]
fn manifest_preserves_custom_date_range() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2024, 6, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 9, 1).unwrap();
    let manifest = CoverageBuilder::new("dave")
        .dates(since, until)
        .build();

    assert_eq!(manifest.window.since, since);
    assert_eq!(manifest.window.until, until);
    Ok(())
}

#[test]
fn manifest_with_slices_records_fetch_details() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    let slices: Vec<CoverageSlice> = windows
        .iter()
        .enumerate()
        .map(|(i, w)| CoverageSlice {
            window: w.clone(),
            query: format!("github prs month {}", i + 1),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec![],
        })
        .collect();

    assert_eq!(slices.len(), 3);
    assert!(slices.iter().all(|s| s.fetched == s.total_count));
    assert!(slices.iter().all(|s| s.incomplete_results == Some(false)));
    Ok(())
}

// ============================================================================
// Completeness checking with missing receipts
// ============================================================================

#[test]
fn incomplete_slice_signals_partial_coverage() -> Result<()> {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };

    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 50,
        fetched: 30,
        incomplete_results: Some(true),
        notes: vec!["Rate limited after 30 results".to_string()],
    };

    assert!(slice.fetched < slice.total_count);
    assert_eq!(slice.incomplete_results, Some(true));
    assert!(!slice.notes.is_empty());
    Ok(())
}

#[test]
fn complete_slice_fetches_all_results() -> Result<()> {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };

    let slice = CoverageSlice {
        window,
        query: "github prs".to_string(),
        total_count: 25,
        fetched: 25,
        incomplete_results: Some(false),
        notes: vec![],
    };

    assert_eq!(slice.fetched, slice.total_count);
    assert_eq!(slice.incomplete_results, Some(false));
    Ok(())
}

#[test]
fn manifest_warnings_track_missing_receipts() -> Result<()> {
    let mut manifest = CoverageBuilder::new("eve")
        .completeness(Completeness::Partial)
        .build();

    manifest.warnings.push("Missing PR #42 from acme/app".to_string());
    manifest.warnings.push("Missing PR #99 from acme/lib".to_string());

    assert_eq!(manifest.warnings.len(), 2);
    assert!(manifest.warnings[0].contains("PR #42"));
    assert!(manifest.warnings[1].contains("PR #99"));
    Ok(())
}

#[test]
fn manifest_slices_with_mixed_completeness() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let windows = month_windows(since, until);

    let slices = [
        CoverageSlice {
            window: windows[0].clone(),
            query: "github prs jan".to_string(),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec![],
        },
        CoverageSlice {
            window: windows[1].clone(),
            query: "github prs feb".to_string(),
            total_count: 50,
            fetched: 30,
            incomplete_results: Some(true),
            notes: vec!["GitHub search limit reached".to_string()],
        },
    ];

    let complete_count = slices.iter().filter(|s| s.fetched == s.total_count).count();
    let incomplete_count = slices.iter().filter(|s| s.fetched < s.total_count).count();

    assert_eq!(complete_count, 1);
    assert_eq!(incomplete_count, 1);
    Ok(())
}

// ============================================================================
// Slicing by date ranges
// ============================================================================

#[test]
fn day_windows_covers_single_day() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 16).unwrap();
    let windows = day_windows(since, until);

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].since, since);
    assert_eq!(windows[0].until, until);
    assert_eq!(window_len_days(&windows[0]), 1);
    Ok(())
}

#[test]
fn day_windows_across_month_boundary() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 30).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 2).unwrap();
    let windows = day_windows(since, until);

    assert_eq!(windows.len(), 3);
    // Jan 30, Jan 31, Feb 1
    assert_eq!(windows[0].since, NaiveDate::from_ymd_opt(2025, 1, 30).unwrap());
    assert_eq!(windows[1].since, NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());
    assert_eq!(windows[2].since, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
    Ok(())
}

#[test]
fn week_windows_short_range_within_single_week() -> Result<()> {
    // Mon Jan 6 to Fri Jan 10 (within one week)
    let since = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 10).unwrap();
    let windows = week_windows(since, until);

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].since, since);
    assert_eq!(windows[0].until, until);
    Ok(())
}

#[test]
fn month_windows_full_quarter() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    assert_eq!(windows.len(), 3);
    assert_eq!(windows[0].since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(windows[0].until, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
    assert_eq!(windows[1].since, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
    assert_eq!(windows[1].until, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
    assert_eq!(windows[2].since, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
    assert_eq!(windows[2].until, NaiveDate::from_ymd_opt(2025, 4, 1).unwrap());
    Ok(())
}

#[test]
fn month_windows_across_year_boundary() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2024, 11, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let windows = month_windows(since, until);

    assert_eq!(windows.len(), 3);
    assert_eq!(windows[0].until, NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
    assert_eq!(windows[1].until, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    assert_eq!(windows[2].until, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
    Ok(())
}

#[test]
fn window_len_days_for_each_window_type() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();

    // Day windows should all be 1 day
    let days = day_windows(since, until);
    assert!(days.iter().all(|w| window_len_days(w) == 1));
    assert_eq!(days.len(), 31);

    // Month window for January: 31 days
    let months = month_windows(since, until);
    assert_eq!(months.len(), 1);
    assert_eq!(window_len_days(&months[0]), 31);

    Ok(())
}

#[test]
fn time_window_contains_covers_range_correctly() -> Result<()> {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 3, 31).unwrap(),
    };

    // Since is inclusive
    assert!(window.contains(NaiveDate::from_ymd_opt(2025, 3, 1).unwrap()));
    // Until is exclusive
    assert!(!window.contains(NaiveDate::from_ymd_opt(2025, 3, 31).unwrap()));
    // Middle date is contained
    assert!(window.contains(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()));
    // Before range
    assert!(!window.contains(NaiveDate::from_ymd_opt(2025, 2, 28).unwrap()));
    // After range
    assert!(!window.contains(NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()));
    Ok(())
}

#[test]
fn all_window_types_empty_for_same_date() -> Result<()> {
    let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();

    assert!(day_windows(d, d).is_empty());
    assert!(week_windows(d, d).is_empty());
    assert!(month_windows(d, d).is_empty());
    Ok(())
}

#[test]
fn all_window_types_empty_for_reversed_range() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();

    assert!(day_windows(since, until).is_empty());
    assert!(week_windows(since, until).is_empty());
    assert!(month_windows(since, until).is_empty());
    Ok(())
}

#[test]
fn coverage_slices_align_with_month_windows() -> Result<()> {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 7, 1).unwrap();
    let windows = month_windows(since, until);

    let slices: Vec<CoverageSlice> = windows
        .iter()
        .map(|w| CoverageSlice {
            window: w.clone(),
            query: "github prs".to_string(),
            total_count: 5,
            fetched: 5,
            incomplete_results: Some(false),
            notes: vec![],
        })
        .collect();

    assert_eq!(slices.len(), 6);
    // Verify slices tile the full range
    assert_eq!(slices.first().unwrap().window.since, since);
    assert_eq!(slices.last().unwrap().window.until, until);
    for pair in slices.windows(2) {
        assert_eq!(pair[0].window.until, pair[1].window.since);
    }
    Ok(())
}
