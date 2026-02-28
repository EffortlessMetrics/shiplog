//! Boundary value tests for date-window partitioning.
//!
//! Covers year boundaries, leap years, month length differences,
//! very long ranges, zero-day ranges, and reversed ranges.

use chrono::{Datelike, NaiveDate, Weekday};
use shiplog_date_windows::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_schema::coverage::TimeWindow;

// ============================================================================
// Year boundaries (Dec 31 → Jan 1)
// ============================================================================

#[test]
fn month_windows_dec31_to_jan1_single_day_across_year() {
    let since = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].since, since);
    assert_eq!(w[0].until, until);
    assert_eq!(window_len_days(&w[0]), 1);
}

#[test]
fn week_windows_across_year_boundary() {
    let since = NaiveDate::from_ymd_opt(2024, 12, 30).unwrap(); // Monday
    let until = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap(); // Monday
    let w = week_windows(since, until);
    // 2024-12-30 is a Monday, 2025-01-06 is the next Monday → single week
    assert_eq!(w.len(), 1);
    assert_eq!(w[0].since, since);
    assert_eq!(w[0].until, until);
    assert_eq!(window_len_days(&w[0]), 7);
}

#[test]
fn day_windows_dec31_to_jan2_spans_year() {
    let since = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].since.year(), 2024);
    assert_eq!(w[0].since.month(), 12);
    assert_eq!(w[0].since.day(), 31);
    assert_eq!(w[1].since.year(), 2025);
    assert_eq!(w[1].since.month(), 1);
    assert_eq!(w[1].since.day(), 1);
}

#[test]
fn month_windows_across_multiple_year_boundaries() {
    let since = NaiveDate::from_ymd_opt(2023, 11, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 15); // Nov 2023 through Jan 2025
    assert_eq!(w.first().unwrap().since, since);
    assert_eq!(w.last().unwrap().until, until);
    // Verify continuity
    for pair in w.windows(2) {
        assert_eq!(pair[0].until, pair[1].since);
    }
}

// ============================================================================
// Leap year (Feb 28/29)
// ============================================================================

#[test]
fn month_windows_feb_leap_year_2024_has_29_days() {
    let since = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 1);
    assert_eq!(window_len_days(&w[0]), 29);
}

#[test]
fn month_windows_feb_non_leap_year_2025_has_28_days() {
    let since = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 1);
    assert_eq!(window_len_days(&w[0]), 28);
}

#[test]
fn day_windows_feb28_to_mar1_leap_year_produces_two_days() {
    let since = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();
    let until = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 2); // Feb 28, Feb 29
    assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
}

#[test]
fn day_windows_feb28_to_mar1_non_leap_year_produces_one_day() {
    let since = NaiveDate::from_ymd_opt(2025, 2, 28).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 1); // Feb 28 only
}

#[test]
fn century_leap_year_2000_feb_has_29_days() {
    let since = NaiveDate::from_ymd_opt(2000, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2000, 3, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(window_len_days(&w[0]), 29);
}

#[test]
fn century_non_leap_year_1900_feb_has_28_days() {
    let since = NaiveDate::from_ymd_opt(1900, 2, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(1900, 3, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(window_len_days(&w[0]), 28);
}

// ============================================================================
// Month boundaries (30 vs 31 days)
// ============================================================================

#[test]
fn month_windows_all_month_lengths_2025() {
    let expected_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (i, &expected) in expected_days.iter().enumerate() {
        let month = (i + 1) as u32;
        let since = NaiveDate::from_ymd_opt(2025, month, 1).unwrap();
        let until = if month == 12 {
            NaiveDate::from_ymd_opt(2026, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(2025, month + 1, 1).unwrap()
        };
        let w = month_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(
            window_len_days(&w[0]),
            expected,
            "month {month} expected {expected} days"
        );
    }
}

#[test]
fn day_windows_across_30_to_31_day_month_boundary() {
    // April (30 days) → May (31 days)
    let since = NaiveDate::from_ymd_opt(2025, 4, 30).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 5, 2).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].since, NaiveDate::from_ymd_opt(2025, 4, 30).unwrap());
    assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2025, 5, 1).unwrap());
}

#[test]
fn day_windows_across_31_to_30_day_month_boundary() {
    // January (31 days) → February (28 days in 2025)
    let since = NaiveDate::from_ymd_opt(2025, 1, 31).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 2).unwrap();
    let w = day_windows(since, until);
    assert_eq!(w.len(), 2);
    assert_eq!(w[0].since, NaiveDate::from_ymd_opt(2025, 1, 31).unwrap());
    assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
}

// ============================================================================
// Very long date ranges (10 years)
// ============================================================================

#[test]
fn month_windows_ten_year_range() {
    let since = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 120); // 10 years × 12 months
    assert_eq!(w.first().unwrap().since, since);
    assert_eq!(w.last().unwrap().until, until);
    // Verify continuity across the full range
    for pair in w.windows(2) {
        assert_eq!(pair[0].until, pair[1].since);
    }
}

#[test]
fn day_windows_ten_year_range_count() {
    let since = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let w = day_windows(since, until);
    let expected = (until - since).num_days() as usize;
    assert_eq!(w.len(), expected);
    assert_eq!(w.first().unwrap().since, since);
    assert_eq!(w.last().unwrap().until, until);
}

#[test]
fn week_windows_ten_year_range_continuity() {
    let since = NaiveDate::from_ymd_opt(2015, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let w = week_windows(since, until);
    assert!(!w.is_empty());
    assert_eq!(w.first().unwrap().since, since);
    assert_eq!(w.last().unwrap().until, until);
    // Total days covered must equal range length
    let total: i64 = w.iter().map(window_len_days).sum();
    assert_eq!(total, (until - since).num_days());
    // Internal boundaries must be Mondays
    for win in w.iter().skip(1) {
        assert_eq!(win.since.weekday(), Weekday::Mon);
    }
}

#[test]
fn window_len_days_ten_year_range() {
    let w = TimeWindow {
        since: NaiveDate::from_ymd_opt(2015, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    };
    // 2015–2024: leap years are 2016, 2020, 2024 → 3 extra days
    assert_eq!(window_len_days(&w), 365 * 10 + 3);
}

// ============================================================================
// Zero-day range (since == until)
// ============================================================================

#[test]
fn zero_day_range_all_functions_return_empty() {
    let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    assert!(month_windows(d, d).is_empty());
    assert!(week_windows(d, d).is_empty());
    assert!(day_windows(d, d).is_empty());
}

#[test]
fn window_len_days_zero_for_zero_range() {
    let d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    assert_eq!(window_len_days(&TimeWindow { since: d, until: d }), 0);
}

// ============================================================================
// Reversed date range (start > end)
// ============================================================================

#[test]
fn reversed_range_all_functions_return_empty() {
    let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    assert!(month_windows(since, until).is_empty());
    assert!(week_windows(since, until).is_empty());
    assert!(day_windows(since, until).is_empty());
}

#[test]
fn reversed_by_one_day_returns_empty() {
    let since = NaiveDate::from_ymd_opt(2025, 3, 2).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    assert!(month_windows(since, until).is_empty());
    assert!(week_windows(since, until).is_empty());
    assert!(day_windows(since, until).is_empty());
}

#[test]
fn window_len_days_negative_for_reversed_window() {
    let w = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    };
    assert!(window_len_days(&w) < 0);
}

// ============================================================================
// Exact boundary dates (start of month/year)
// ============================================================================

#[test]
fn month_windows_exact_calendar_year() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2026, 1, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 12);
    let total: i64 = w.iter().map(window_len_days).sum();
    assert_eq!(total, 365); // 2025 is not a leap year
}

#[test]
fn month_windows_exact_leap_year() {
    let since = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let w = month_windows(since, until);
    assert_eq!(w.len(), 12);
    let total: i64 = w.iter().map(window_len_days).sum();
    assert_eq!(total, 366); // 2024 is a leap year
}

#[test]
fn week_windows_exact_monday_to_monday() {
    // 2025-01-06 is Monday, 2025-01-13 is Monday
    let since = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 13).unwrap();
    assert_eq!(since.weekday(), Weekday::Mon);
    assert_eq!(until.weekday(), Weekday::Mon);
    let w = week_windows(since, until);
    assert_eq!(w.len(), 1);
    assert_eq!(window_len_days(&w[0]), 7);
}
