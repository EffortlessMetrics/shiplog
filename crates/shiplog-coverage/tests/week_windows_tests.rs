use chrono::{Datelike, NaiveDate, Weekday};
use shiplog_coverage::week_windows;

#[test]
fn week_windows_is_empty_for_empty_range() {
    let t = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    assert!(week_windows(t, t).is_empty());
}

#[test]
fn week_windows_is_non_empty_for_non_empty_range() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
    let windows = week_windows(since, until);
    assert!(!windows.is_empty(), "non-empty range must yield windows");
}

#[test]
fn week_windows_produces_only_positive_length_windows() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    let windows = week_windows(since, until);

    assert!(!windows.is_empty());
    assert!(
        windows.iter().all(|w| w.since < w.until),
        "all week windows must have positive length"
    );
}

#[test]
fn week_windows_partitions_range_without_gaps_or_overlaps() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let windows = week_windows(since, until);

    assert!(!windows.is_empty());
    assert_eq!(windows.first().unwrap().since, since, "covers from since");
    assert_eq!(windows.last().unwrap().until, until, "covers to until");

    for pair in windows.windows(2) {
        assert_eq!(pair[0].until, pair[1].since, "no gaps/overlaps");
    }
}

#[test]
fn week_windows_align_internal_boundaries_to_monday() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 29).unwrap();
    let windows = week_windows(since, until);

    assert!(windows.len() >= 2);

    for w in &windows[..windows.len() - 1] {
        assert_eq!(w.until.weekday(), Weekday::Mon);
    }
}
