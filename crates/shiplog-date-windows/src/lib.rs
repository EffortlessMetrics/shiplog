//! Date-window partition primitives used by shiplog ingestion logic.

use chrono::{Datelike, NaiveDate, Weekday};
use shiplog_schema::coverage::TimeWindow;

/// Split a half-open date range into month-start anchored windows.
///
/// # Examples
///
/// ```
/// use shiplog_date_windows::month_windows;
/// use chrono::NaiveDate;
///
/// let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
/// let until = NaiveDate::from_ymd_opt(2025, 3, 2).unwrap();
/// let windows = month_windows(since, until);
///
/// assert_eq!(windows.len(), 3);
/// assert_eq!(windows[0].since, since);
/// assert_eq!(windows[1].since, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
/// ```
pub fn month_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;

    while cursor < until {
        let next = next_month_start(cursor);
        let end = std::cmp::min(next, until);
        assert!(
            end > cursor,
            "month_windows must make forward progress (until is exclusive)"
        );
        out.push(TimeWindow {
            since: cursor,
            until: end,
        });
        cursor = end;
    }

    out
}

/// Split a half-open date range into Monday-started week windows.
///
/// # Examples
///
/// ```
/// use shiplog_date_windows::week_windows;
/// use chrono::NaiveDate;
///
/// let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
/// let until = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
/// let windows = week_windows(since, until);
///
/// assert!(!windows.is_empty());
/// // Internal boundaries fall on Mondays
/// ```
pub fn week_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;

    while cursor < until {
        let next = next_week_start(cursor, Weekday::Mon);
        let end = std::cmp::min(next, until);
        assert!(
            end > cursor,
            "week_windows must make forward progress (until is exclusive)"
        );
        out.push(TimeWindow {
            since: cursor,
            until: end,
        });
        cursor = end;
    }

    out
}

/// Split a half-open date range into day windows.
///
/// # Examples
///
/// ```
/// use shiplog_date_windows::day_windows;
/// use chrono::NaiveDate;
///
/// let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
/// let until = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();
/// let days = day_windows(since, until);
///
/// assert_eq!(days.len(), 3); // Jan 1, 2, 3
/// ```
pub fn day_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;
    while cursor < until {
        let end = std::cmp::min(cursor.succ_opt().unwrap_or(until), until);
        assert!(
            end > cursor,
            "day_windows must make forward progress (until is exclusive)"
        );
        out.push(TimeWindow {
            since: cursor,
            until: end,
        });
        cursor = end;
    }
    out
}

/// Number of days covered by a window.
///
/// # Examples
///
/// ```
/// use shiplog_date_windows::window_len_days;
/// use shiplog_schema::coverage::TimeWindow;
/// use chrono::NaiveDate;
///
/// let w = TimeWindow {
///     since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
///     until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
/// };
/// assert_eq!(window_len_days(&w), 31);
/// ```
#[must_use]
pub fn window_len_days(w: &TimeWindow) -> i64 {
    (w.until - w.since).num_days()
}

fn next_month_start(d: NaiveDate) -> NaiveDate {
    let (y, m) = (d.year(), d.month());
    let (ny, nm) = if m == 12 { (y + 1, 1) } else { (y, m + 1) };
    NaiveDate::from_ymd_opt(ny, nm, 1).unwrap()
}

fn next_week_start(d: NaiveDate, start: Weekday) -> NaiveDate {
    let mut cursor = d;
    loop {
        cursor = cursor.succ_opt().unwrap();
        if cursor.weekday() == start {
            return cursor;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_windows_splits_on_month_boundaries() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 2).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].since, since);
        assert_eq!(w[0].until, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
        assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
        assert_eq!(w[1].until, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
        assert_eq!(w[2].since, NaiveDate::from_ymd_opt(2025, 3, 1).unwrap());
        assert_eq!(w[2].until, until);
    }

    #[test]
    fn day_windows_counts_days() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();
        let w = day_windows(since, until);
        assert_eq!(w.len(), 3);
        assert_eq!(window_len_days(&w[0]), 1);
    }

    // --- Empty and inverted range edge cases ---

    #[test]
    fn month_windows_empty_when_since_equals_until() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        assert!(month_windows(d, d).is_empty());
    }

    #[test]
    fn week_windows_empty_when_since_equals_until() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        assert!(week_windows(d, d).is_empty());
    }

    #[test]
    fn day_windows_empty_when_since_equals_until() {
        let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        assert!(day_windows(d, d).is_empty());
    }

    #[test]
    fn month_windows_empty_when_since_after_until() {
        let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(month_windows(since, until).is_empty());
    }

    #[test]
    fn week_windows_empty_when_since_after_until() {
        let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(week_windows(since, until).is_empty());
    }

    #[test]
    fn day_windows_empty_when_since_after_until() {
        let since = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        assert!(day_windows(since, until).is_empty());
    }

    // --- Single day range ---

    #[test]
    fn day_windows_single_day() {
        let since = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 16).unwrap();
        let w = day_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].since, since);
        assert_eq!(w[0].until, until);
        assert_eq!(window_len_days(&w[0]), 1);
    }

    #[test]
    fn month_windows_single_day() {
        let since = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 16).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].since, since);
        assert_eq!(w[0].until, until);
    }

    #[test]
    fn week_windows_single_day() {
        let since = NaiveDate::from_ymd_opt(2025, 3, 15).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 16).unwrap();
        let w = week_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].since, since);
        assert_eq!(w[0].until, until);
    }

    // --- Leap year edge cases ---

    #[test]
    fn day_windows_across_leap_day() {
        let since = NaiveDate::from_ymd_opt(2024, 2, 28).unwrap();
        let until = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let w = day_windows(since, until);
        assert_eq!(w.len(), 2); // Feb 28 and Feb 29
        assert_eq!(w[0].since, NaiveDate::from_ymd_opt(2024, 2, 28).unwrap());
        assert_eq!(w[0].until, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
        assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2024, 2, 29).unwrap());
        assert_eq!(w[1].until, NaiveDate::from_ymd_opt(2024, 3, 1).unwrap());
    }

    #[test]
    fn day_windows_across_non_leap_feb() {
        let since = NaiveDate::from_ymd_opt(2025, 2, 28).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let w = day_windows(since, until);
        assert_eq!(w.len(), 1); // No Feb 29 in 2025
    }

    #[test]
    fn month_windows_leap_year_february() {
        let since = NaiveDate::from_ymd_opt(2024, 2, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2024, 3, 1).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(window_len_days(&w[0]), 29); // Leap year: 29 days in Feb
    }

    #[test]
    fn month_windows_non_leap_year_february() {
        let since = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 3, 1).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 1);
        assert_eq!(window_len_days(&w[0]), 28); // Non-leap: 28 days
    }

    // --- Year boundary ---

    #[test]
    fn month_windows_across_year_boundary() {
        let since = NaiveDate::from_ymd_opt(2024, 12, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].since, NaiveDate::from_ymd_opt(2024, 12, 1).unwrap());
        assert_eq!(w[0].until, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
        assert_eq!(w[1].until, NaiveDate::from_ymd_opt(2025, 2, 1).unwrap());
    }

    #[test]
    fn day_windows_across_year_boundary() {
        let since = NaiveDate::from_ymd_opt(2024, 12, 31).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 1, 2).unwrap();
        let w = day_windows(since, until);
        assert_eq!(w.len(), 2);
        assert_eq!(w[0].since, NaiveDate::from_ymd_opt(2024, 12, 31).unwrap());
        assert_eq!(w[1].since, NaiveDate::from_ymd_opt(2025, 1, 1).unwrap());
    }

    // --- Week windows on Monday boundaries ---

    #[test]
    fn week_windows_internal_boundaries_are_mondays() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(); // Wednesday
        let until = NaiveDate::from_ymd_opt(2025, 1, 22).unwrap(); // Wednesday
        let w = week_windows(since, until);
        // Internal boundaries (all except first.since and last.until) should be Mondays
        for (i, win) in w.iter().enumerate().skip(1) {
            assert_eq!(
                win.since.weekday(),
                Weekday::Mon,
                "Window {} starts on {:?}, expected Monday",
                i,
                win.since.weekday()
            );
        }
    }

    // --- Month windows starting on first of month ---

    #[test]
    fn month_windows_exact_month_boundaries() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let w = month_windows(since, until);
        assert_eq!(w.len(), 3);
        assert_eq!(window_len_days(&w[0]), 31); // January
        assert_eq!(window_len_days(&w[1]), 28); // February 2025
        assert_eq!(window_len_days(&w[2]), 31); // March
    }

    // --- window_len_days ---

    #[test]
    fn window_len_days_zero_for_same_dates() {
        let d = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let w = TimeWindow { since: d, until: d };
        assert_eq!(window_len_days(&w), 0);
    }

    #[test]
    fn window_len_days_full_year() {
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        };
        assert_eq!(window_len_days(&w), 366); // 2024 is a leap year
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_month_windows_q1_2025() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
        let w = month_windows(since, until);
        insta::assert_yaml_snapshot!("month_windows_q1_2025", w);
    }

    #[test]
    fn snapshot_week_windows_jan_2025() {
        let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
        let until = NaiveDate::from_ymd_opt(2025, 2, 1).unwrap();
        let w = week_windows(since, until);
        insta::assert_yaml_snapshot!("week_windows_jan_2025", w);
    }

    #[test]
    fn snapshot_day_windows_leap_feb_end() {
        let since = NaiveDate::from_ymd_opt(2024, 2, 27).unwrap();
        let until = NaiveDate::from_ymd_opt(2024, 3, 2).unwrap();
        let w = day_windows(since, until);
        insta::assert_yaml_snapshot!("day_windows_leap_feb_end", w);
    }
}
