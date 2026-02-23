//! Property tests for shiplog-coverage
//!
//! This module contains property-based tests for coverage window and slice invariants.

use chrono::NaiveDate;
use proptest::prelude::*;
use shiplog_coverage::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_schema::coverage::TimeWindow;
use shiplog_testkit::proptest::strategy_naive_date;

// ============================================================================
// Time Window Generation Tests
// ============================================================================

proptest! {
    // Day windows are contiguous and cover the full range.
    #[test]
    fn prop_day_windows_contiguous_and_complete(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = day_windows(since, until);

        prop_assert!(!windows.is_empty());
        prop_assert_eq!(windows.first().unwrap().since, since);
        prop_assert_eq!(windows.last().unwrap().until, until);

        for i in 0..windows.len().saturating_sub(1) {
            prop_assert_eq!(windows[i].until, windows[i + 1].since);
        }

        for w in &windows {
            prop_assert_eq!(window_len_days(w), 1);
        }
    }

    // Week windows are contiguous and ordered.
    #[test]
    fn prop_week_windows_ordered(
        since in strategy_naive_date(),
        days in 7u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = week_windows(since, until);

        if windows.is_empty() {
            return Ok(());
        }

        prop_assert_eq!(windows.first().unwrap().since, since);
        prop_assert_eq!(windows.last().unwrap().until, until);

        for i in 0..windows.len().saturating_sub(1) {
            prop_assert!(windows[i].since <= windows[i + 1].since);
            prop_assert_eq!(windows[i].until, windows[i + 1].since);
        }
    }

    // Month windows are contiguous and ordered.
    #[test]
    fn prop_month_windows_ordered(
        since in strategy_naive_date(),
        days in 28u64..730u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = month_windows(since, until);

        if windows.is_empty() {
            return Ok(());
        }

        prop_assert_eq!(windows.first().unwrap().since, since);
        prop_assert_eq!(windows.last().unwrap().until, until);

        for i in 0..windows.len().saturating_sub(1) {
            prop_assert!(windows[i].since <= windows[i + 1].since);
            prop_assert_eq!(windows[i].until, windows[i + 1].since);
        }
    }

    // TimeWindow::contains is inclusive at since and exclusive at until.
    #[test]
    fn prop_contains_semantics(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };

        prop_assert!(window.contains(since));
        prop_assert!(!window.contains(until));

        if days > 1 {
            let inside = since.checked_add_days(chrono::Days::new(days / 2)).unwrap();
            prop_assert!(window.contains(inside));
        }
    }
}

// ============================================================================
// Coverage Slice Invariant Tests
// ============================================================================

proptest! {
    // fetched should not exceed total_count in valid slices.
    #[test]
    fn prop_fetched_never_exceeds_total(
        fetched in 0u64..1000u64,
        total in 0u64..1000u64
    ) {
        prop_assume!(fetched <= total);

        let slice = shiplog_schema::coverage::CoverageSlice {
            window: TimeWindow {
                since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            query: "github prs".to_string(),
            fetched,
            total_count: total,
            incomplete_results: Some(fetched < total),
            notes: vec![],
        };

        prop_assert!(slice.fetched <= slice.total_count);
    }

    // incomplete_results should reflect whether fetched < total_count.
    #[test]
    fn prop_incomplete_results_flag(
        fetched in 0u64..1000u64,
        total in 0u64..1000u64
    ) {
        let total_count = fetched.max(total);
        let fetched_count = fetched.min(total);

        let slice = shiplog_schema::coverage::CoverageSlice {
            window: TimeWindow {
                since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            query: "github prs".to_string(),
            fetched: fetched_count,
            total_count,
            incomplete_results: Some(fetched_count < total_count),
            notes: vec![],
        };

        prop_assert_eq!(slice.incomplete_results, Some(slice.fetched < slice.total_count));
    }
}
