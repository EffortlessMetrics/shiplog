use chrono::NaiveDate;
use shiplog_coverage::month_windows;

#[test]
fn month_windows_is_empty_for_empty_range() {
    let t = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let windows = month_windows(t, t);
    assert!(windows.is_empty(), "empty range must produce no windows");
}

#[test]
fn month_windows_produces_only_positive_length_windows() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
    let windows = month_windows(since, until);

    assert!(!windows.is_empty());
    assert!(
        windows.iter().all(|w| w.since < w.until),
        "month windows must all have positive length"
    );
}

#[test]
fn month_windows_partitions_range_without_gaps_or_overlaps() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 2).unwrap();
    let windows = month_windows(since, until);

    assert!(!windows.is_empty());
    assert_eq!(windows.first().unwrap().since, since, "covers from since");
    assert_eq!(windows.last().unwrap().until, until, "covers to until");

    for pair in windows.windows(2) {
        assert_eq!(pair[0].until, pair[1].since, "no gaps or overlaps");
    }
}

#[test]
fn month_windows_does_not_emit_tail_window_at_exact_month_boundary() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
    let windows = month_windows(since, until);

    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].since, since);
    assert_eq!(windows[0].until, until);
    assert!(
        windows.iter().all(|w| w.since < w.until),
        "no empty tail window at boundary"
    );
}
