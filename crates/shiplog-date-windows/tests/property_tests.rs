//! Property tests for shiplog-date-windows.

use chrono::{Datelike, NaiveDate};
use proptest::prelude::*;
use shiplog_date_windows::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_schema::coverage::TimeWindow;
use shiplog_testkit::proptest::strategy_naive_date;

proptest! {
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
            prop_assert!(windows[i].since < windows[i].until);
        }

        for w in &windows {
            prop_assert_eq!(window_len_days(w), 1);
        }
    }

    #[test]
    fn prop_week_windows_partitions_with_no_gaps(
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
            if i + 1 < windows.len() - 1 {
                prop_assert_eq!(windows[i + 1].since.weekday(), chrono::Weekday::Mon);
            }
        }
    }

    #[test]
    fn prop_month_windows_partition_to_month_boundaries(
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

        let sum = windows
            .iter()
            .map(|w| (w.until - w.since).num_days())
            .sum::<i64>();
        prop_assert_eq!(sum, (until - since).num_days());

        for i in 0..windows.len().saturating_sub(1) {
            prop_assert_eq!(windows[i].until, windows[i + 1].since);
        }
    }

    #[test]
    fn prop_time_window_contains_semantics(
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
// Additional Window Partition Property Tests
// ============================================================================

proptest! {
    // Sum of day window lengths equals the total number of days in the range.
    #[test]
    fn prop_day_windows_sum_equals_range(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = day_windows(since, until);
        let total: i64 = windows.iter().map(window_len_days).sum();
        prop_assert_eq!(total, (until - since).num_days());
    }

    // Every window produced by any partitioner has positive length (since < until).
    #[test]
    fn prop_all_windows_positive_length(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        for w in day_windows(since, until).iter()
            .chain(week_windows(since, until).iter())
            .chain(month_windows(since, until).iter())
        {
            prop_assert!(w.since < w.until, "window {:?}..{:?} has non-positive length", w.since, w.until);
        }
    }

    // Number of day windows equals the number of calendar days in the range.
    #[test]
    fn prop_day_window_count_equals_days(
        since in strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = day_windows(since, until);
        prop_assert_eq!(windows.len() as i64, (until - since).num_days());
    }

    // Week windows total days equals the full date range span.
    #[test]
    fn prop_week_windows_sum_equals_range(
        since in strategy_naive_date(),
        days in 7u64..365u64
    ) {
        let until: NaiveDate = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let windows = week_windows(since, until);
        if !windows.is_empty() {
            let total: i64 = windows.iter().map(window_len_days).sum();
            prop_assert_eq!(total, (until - since).num_days());
        }
    }
}
