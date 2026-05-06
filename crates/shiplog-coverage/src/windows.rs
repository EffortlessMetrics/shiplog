//! Date-window partition primitives used by coverage and ingestion logic.

use chrono::{Datelike, NaiveDate, Weekday};
use shiplog_schema::coverage::TimeWindow;

/// Split a half-open date range into month-start anchored windows.
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use shiplog_coverage::month_windows;
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
/// use chrono::NaiveDate;
/// use shiplog_coverage::week_windows;
///
/// let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
/// let until = NaiveDate::from_ymd_opt(2025, 1, 15).unwrap();
/// let windows = week_windows(since, until);
///
/// assert!(!windows.is_empty());
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
/// use chrono::NaiveDate;
/// use shiplog_coverage::day_windows;
///
/// let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
/// let until = NaiveDate::from_ymd_opt(2025, 1, 4).unwrap();
/// let days = day_windows(since, until);
///
/// assert_eq!(days.len(), 3);
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
/// use chrono::NaiveDate;
/// use shiplog_coverage::window_len_days;
/// use shiplog_schema::coverage::TimeWindow;
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
