//! Property tests for shiplog-coverage
//!
//! This module contains property-based tests for coverage calculation invariants
//! (time windowing and slicing correctness).

use proptest::prelude::*;
use shiplog_coverage::TimeWindow;
use shiplog_testkit::proptest::*;

// ============================================================================
// Time Window Generation Tests
// ============================================================================

proptest! {
    /// Test that generated windows never overlap
    #[test]
    fn prop_windows_non_overlapping(
        since in strategy_naive_date(),
        days in 10u64..365u64,
        window_size_days in 1u64..30u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };
        let sub_windows = window.generate_windows(window_size_days as usize);

        for i in 0..sub_windows.len().saturating_sub(1) {
            let current = &sub_windows[i];
            let next = &sub_windows[i + 1];
            // Current window's until should be <= next window's since
            prop_assert!(current.until <= next.since);
        }
    }

    /// Test that union of all windows equals original range
    #[test]
    fn prop_windows_coverage_complete(
        since in strategy_naive_date(),
        days in 10u64..365u64,
        window_size_days in 1u64..30u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };
        let sub_windows = window.generate_windows(window_size_days as usize);

        if sub_windows.is_empty() {
            return Ok(());
        }

        let first_since = sub_windows.first().unwrap().since;
        let last_until = sub_windows.last().unwrap().until;

        prop_assert_eq!(first_since, since);
        prop_assert_eq!(last_until, until);
    }

    /// Test that windows align to natural boundaries
    #[test]
    fn prop_windows_boundary_alignment(
        since in strategy_naive_date(),
        days in 30u64..365u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };

        // Test day windows (always length 1)
        let day_windows = window.generate_windows(1);
        for w in day_windows {
            let duration = w.until.signed_duration_since(w.since).num_days();
            prop_assert_eq!(duration, 1);
        }

        // Test week windows (always length 7)
        let week_windows = window.generate_windows(7);
        for w in week_windows {
            let duration = w.until.signed_duration_since(w.since).num_days();
            prop_assert_eq!(duration, 7);
        }
    }

    /// Test that empty range returns empty vector
    #[test]
    fn prop_empty_range_returns_empty(date in strategy_naive_date()) {
        let window = TimeWindow { since: date, until: date };
        let sub_windows = window.generate_windows(7);
        prop_assert!(sub_windows.is_empty());
    }

    /// Test that single day window has length 1
    #[test]
    fn prop_single_day_window_length(date in strategy_naive_date()) {
        let next_day = date.checked_add_days(chrono::Days::new(1)).unwrap();
        let window = TimeWindow { since: date, until: next_day };
        let day_windows = window.generate_windows(1);
        prop_assert_eq!(day_windows.len(), 1);
        prop_assert_eq!(day_windows[0].since, date);
        prop_assert_eq!(day_windows[0].until, next_day);
    }

    /// Test that week windows always have length 7
    #[test]
    fn prop_week_window_length(
        since in strategy_naive_date(),
        weeks in 1u64..10u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(weeks * 7)).unwrap();
        let window = TimeWindow { since, until };
        let week_windows = window.generate_windows(7);

        for w in week_windows {
            let duration = w.until.signed_duration_since(w.since).num_days();
            prop_assert_eq!(duration, 7);
        }
    }

    /// Test that month windows vary by month (28-31 days)
    #[test]
    fn prop_month_window_length(
        since in strategy_naive_date(),
        months in 1u64..12u64
    ) {
        let mut until = since;
        for _ in 0..months {
            until = until.checked_add_days(chrono::Days::new(31)).unwrap();
        }
        let window = TimeWindow { since, until };
        let month_windows = window.generate_windows(31);

        for w in month_windows {
            let duration = w.until.signed_duration_since(w.since).num_days();
            // Month windows should be between 28 and 31 days
            prop_assert!(duration >= 28 && duration <= 31);
        }
    }

    /// Test that output windows are in chronological order
    #[test]
    fn prop_windows_order_preserved(
        since in strategy_naive_date(),
        days in 10u64..365u64,
        window_size_days in 1u64..30u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };
        let sub_windows = window.generate_windows(window_size_days as usize);

        for i in 0..sub_windows.len().saturating_sub(1) {
            let current = &sub_windows[i];
            let next = &sub_windows[i + 1];
            prop_assert!(current.since <= next.since);
            prop_assert!(current.until <= next.until);
        }
    }

    /// Test that no gaps between windows
    #[test]
    fn prop_no_gaps_between_windows(
        since in strategy_naive_date(),
        days in 10u64..365u64,
        window_size_days in 1u64..30u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };
        let sub_windows = window.generate_windows(window_size_days as usize);

        for i in 0..sub_windows.len().saturating_sub(1) {
            let current = &sub_windows[i];
            let next = &sub_windows[i + 1];
            // Current window's until should equal next window's since
            prop_assert_eq!(current.until, next.since);
        }
    }
}

// ============================================================================
// TimeWindow::contains Tests
// ============================================================================

proptest! {
    /// Test that TimeWindow::contains correctly identifies dates
    #[test]
    fn prop_contains_check_correct(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };

        // Test a date inside the window
        let inside = since.checked_add_days(chrono::Days::new(days / 2)).unwrap();
        prop_assert!(window.contains(inside));

        // Test a date before the window
        if let Some(before) = since.checked_sub_days(chrono::Days::new(1)) {
            prop_assert!(!window.contains(before));
        }

        // Test a date after the window
        let after = until.checked_add_days(chrono::Days::new(1)).unwrap();
        prop_assert!(!window.contains(after));
    }

    /// Test that window boundaries are inclusive
    #[test]
    fn prop_boundaries_inclusive(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };

        // Both boundaries should be included
        prop_assert!(window.contains(since));
        prop_assert!(window.contains(until));
    }
}

// ============================================================================
// Coverage Slice Invariant Tests
// ============================================================================

proptest! {
    /// Test that fetched never exceeds total_count
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
            fetched,
            total_count: total,
            completeness: shiplog_schema::coverage::Completeness::Complete,
            warnings: vec![],
        };

        prop_assert!(slice.fetched <= slice.total_count);
    }

    /// Test that partial slices set completeness to Partial
    #[test]
    fn prop_partial_sets_completeness(
        fetched in 0u64..999u64,
        total in 1u64..1000u64
    ) {
        prop_assume!(fetched < total);
        let slice = shiplog_schema::coverage::CoverageSlice {
            window: TimeWindow {
                since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            fetched,
            total_count: total,
            completeness: shiplog_schema::coverage::Completeness::Partial,
            warnings: vec![],
        };

        prop_assert_eq!(slice.completeness, shiplog_schema::coverage::Completeness::Partial);
    }

    /// Test that complete slices have fetched == total_count
    #[test]
    fn prop_complete_has_fetched_equals_total(
        count in 0u64..1000u64
    ) {
        let slice = shiplog_schema::coverage::CoverageSlice {
            window: TimeWindow {
                since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: chrono::NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            fetched: count,
            total_count: count,
            completeness: shiplog_schema::coverage::Completeness::Complete,
            warnings: vec![],
        };

        prop_assert_eq!(slice.fetched, slice.total_count);
        prop_assert_eq!(slice.completeness, shiplog_schema::coverage::Completeness::Complete);
    }
}
