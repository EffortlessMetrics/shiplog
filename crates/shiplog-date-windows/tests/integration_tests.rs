//! Integration tests for shiplog-date-windows.

use chrono::NaiveDate;
use shiplog_date_windows::{day_windows, month_windows, week_windows};
use shiplog_schema::coverage::TimeWindow;

#[test]
fn month_windows_partitions_and_starts_from_input_boundary() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 2).unwrap();
    let windows = month_windows(since, until);

    assert!(!windows.is_empty());
    assert_eq!(windows.first().unwrap().since, since);
    assert_eq!(windows.last().unwrap().until, until);

    for pair in windows.windows(2) {
        assert_eq!(pair[0].until, pair[1].since);
        assert!(pair[0].since < pair[0].until);
    }
    assert!(windows.last().unwrap().since < windows.last().unwrap().until);
}

#[test]
fn day_windows_have_unit_length() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 6).unwrap();
    let windows = day_windows(since, until);

    assert_eq!(windows.len(), 5);
    assert!(windows.iter().all(|w| w.since < w.until));
}

#[test]
fn window_count_is_consistent_with_sum_of_lengths() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();

    let windows = week_windows(since, until);
    let total_len: i64 = windows.iter().map(|w| (w.until - w.since).num_days()).sum();

    assert_eq!(total_len, (until - since).num_days());
    assert!(!windows.is_empty());
}

#[test]
fn empty_range_yields_no_windows() {
    let t = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    assert!(month_windows(t, t).is_empty());
    assert!(week_windows(t, t).is_empty());
    assert!(day_windows(t, t).is_empty());
}

#[test]
fn windows_cover_expected_schema_window() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 3).unwrap(),
    };

    let windows = day_windows(window.since, window.until);
    assert_eq!(windows[0].since, window.since);
    assert_eq!(windows.last().unwrap().until, window.until);
}
