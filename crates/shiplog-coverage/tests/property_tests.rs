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

// ============================================================================
// Coverage Percentage Invariant Tests
// ============================================================================

proptest! {
    // Coverage ratio (fetched / total_count) is always in [0.0, 1.0] for valid slices.
    #[test]
    fn prop_coverage_ratio_bounded(
        fetched in 0u64..10000u64,
        extra in 0u64..10000u64
    ) {
        let total_count = fetched + extra;
        if total_count > 0 {
            let ratio = fetched as f64 / total_count as f64;
            let pct = ratio * 100.0;
            prop_assert!((0.0..=100.0).contains(&pct),
                "Coverage percentage {} out of [0, 100] range (fetched={}, total={})",
                pct, fetched, total_count);
        }
    }

    // Adding more fetched items never decreases coverage ratio.
    #[test]
    fn prop_more_fetched_never_decreases_coverage(
        base_fetched in 0u64..5000u64,
        additional in 1u64..5000u64,
        total_count in 1u64..10000u64
    ) {
        let capped_base = base_fetched.min(total_count);
        let capped_more = (capped_base + additional).min(total_count);

        let ratio_base = capped_base as f64 / total_count as f64;
        let ratio_more = capped_more as f64 / total_count as f64;

        prop_assert!(ratio_more >= ratio_base,
            "Adding fetched items decreased coverage: {} -> {} (base={}, more={}, total={})",
            ratio_base, ratio_more, capped_base, capped_more, total_count);
    }

    // Zero fetched with positive total always gives 0% coverage.
    #[test]
    fn prop_zero_fetched_is_zero_coverage(total_count in 1u64..10000u64) {
        let ratio = 0u64 as f64 / total_count as f64;
        prop_assert_eq!(ratio, 0.0);
    }
}

// ============================================================================
// Window Length Invariant Tests
// ============================================================================

proptest! {
    // All generated day windows have positive length.
    #[test]
    fn prop_day_windows_positive_length(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = day_windows(since, until);
        for w in &windows {
            prop_assert!(window_len_days(w) > 0, "Day window has non-positive length");
        }
    }

    // Number of day windows equals the day span.
    #[test]
    fn prop_day_windows_count_equals_span(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = day_windows(since, until);
        prop_assert_eq!(
            windows.len() as u64, days,
            "Day window count {} != day span {}", windows.len(), days
        );
    }

    // All generated week windows have positive length.
    #[test]
    fn prop_week_windows_positive_length(
        since in strategy_naive_date(),
        days in 7u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = week_windows(since, until);
        for w in &windows {
            prop_assert!(window_len_days(w) > 0, "Week window has non-positive length");
        }
    }

    // All generated month windows have positive length.
    #[test]
    fn prop_month_windows_positive_length(
        since in strategy_naive_date(),
        days in 28u64..730u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = month_windows(since, until);
        for w in &windows {
            prop_assert!(window_len_days(w) > 0, "Month window has non-positive length");
        }
    }

    // Empty range (since == until) produces no windows.
    #[test]
    fn prop_empty_range_no_day_windows(since in strategy_naive_date()) {
        let windows = day_windows(since, since);
        prop_assert!(windows.is_empty(), "Empty range should produce no day windows");
    }

    // Empty range produces no week windows.
    #[test]
    fn prop_empty_range_no_week_windows(since in strategy_naive_date()) {
        let windows = week_windows(since, since);
        prop_assert!(windows.is_empty(), "Empty range should produce no week windows");
    }

    // Empty range produces no month windows.
    #[test]
    fn prop_empty_range_no_month_windows(since in strategy_naive_date()) {
        let windows = month_windows(since, since);
        prop_assert!(windows.is_empty(), "Empty range should produce no month windows");
    }
}
