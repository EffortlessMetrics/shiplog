//! Date-window partition primitives used by coverage and ingestion logic.

use chrono::{Datelike, NaiveDate, Weekday};
use shiplog::schema::coverage::TimeWindow;

/// Split a half-open date range into month-start anchored windows.
///
/// # Examples
///
/// ```
/// use chrono::NaiveDate;
/// use shiplog::coverage::month_windows;
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
/// use shiplog::coverage::week_windows;
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
/// use shiplog::coverage::day_windows;
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
/// use shiplog::coverage::window_len_days;
/// use shiplog::schema::coverage::TimeWindow;
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

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        // unwrap_or_default keeps this helper panic-family clean; invalid inputs
        // surface as wrong-date assertion failures in callers, which is fine for
        // hand-picked constants.
        NaiveDate::from_ymd_opt(y, m, day).unwrap_or_default()
    }

    fn assert_contiguous(windows: &[TimeWindow], since: NaiveDate, until: NaiveDate) {
        assert!(!windows.is_empty(), "expected at least one window");
        assert_eq!(windows[0].since, since, "first.since");
        assert_eq!(windows[windows.len() - 1].until, until, "last.until");
        for w in windows {
            assert!(w.since < w.until, "forward progress: {:?}", w);
        }
        for pair in windows.windows(2) {
            assert_eq!(pair[0].until, pair[1].since, "contiguity gap: {:?}", pair);
        }
    }

    // ---- month_windows ----

    #[test]
    fn month_windows_since_equals_until_is_empty() {
        let day = d(2025, 1, 15);
        assert!(month_windows(day, day).is_empty());
    }

    #[test]
    fn month_windows_since_after_until_is_empty() {
        let since = d(2025, 3, 1);
        let until = d(2025, 2, 1);
        assert!(month_windows(since, until).is_empty());
    }

    #[test]
    fn month_windows_single_window_within_one_month() {
        let since = d(2025, 1, 5);
        let until = d(2025, 1, 20);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, until);
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn month_windows_starts_on_first_of_month_full_month() {
        let since = d(2025, 1, 1);
        let until = d(2025, 2, 1);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, until);
    }

    #[test]
    fn month_windows_starts_on_first_spans_multiple_months() {
        let since = d(2025, 1, 1);
        let until = d(2025, 4, 1);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 3);
        assert_eq!(
            windows[0],
            TimeWindow {
                since: d(2025, 1, 1),
                until: d(2025, 2, 1)
            }
        );
        assert_eq!(
            windows[1],
            TimeWindow {
                since: d(2025, 2, 1),
                until: d(2025, 3, 1)
            }
        );
        assert_eq!(
            windows[2],
            TimeWindow {
                since: d(2025, 3, 1),
                until: d(2025, 4, 1)
            }
        );
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn month_windows_mid_month_to_mid_month_three_windows() {
        let since = d(2025, 1, 15);
        let until = d(2025, 3, 2);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 3);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, d(2025, 2, 1));
        assert_eq!(windows[1].since, d(2025, 2, 1));
        assert_eq!(windows[1].until, d(2025, 3, 1));
        assert_eq!(windows[2].since, d(2025, 3, 1));
        assert_eq!(windows[2].until, until);
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn month_windows_crosses_year_boundary() {
        let since = d(2024, 12, 20);
        let until = d(2025, 2, 10);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 3);
        assert_eq!(
            windows[0],
            TimeWindow {
                since: d(2024, 12, 20),
                until: d(2025, 1, 1)
            }
        );
        assert_eq!(
            windows[1],
            TimeWindow {
                since: d(2025, 1, 1),
                until: d(2025, 2, 1)
            }
        );
        assert_eq!(
            windows[2],
            TimeWindow {
                since: d(2025, 2, 1),
                until: d(2025, 2, 10)
            }
        );
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn month_windows_leap_year_february() {
        // 2024 is a leap year; Feb has 29 days.
        let since = d(2024, 2, 1);
        let until = d(2024, 3, 1);
        let windows = month_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(window_len_days(&windows[0]), 29);
    }

    // ---- week_windows ----

    #[test]
    fn week_windows_since_equals_until_is_empty() {
        let day = d(2025, 1, 1);
        assert!(week_windows(day, day).is_empty());
    }

    #[test]
    fn week_windows_since_after_until_is_empty() {
        assert!(week_windows(d(2025, 3, 1), d(2025, 2, 1)).is_empty());
    }

    #[test]
    fn week_windows_single_day_range_is_one_window() {
        let since = d(2025, 1, 1); // Wednesday
        let until = d(2025, 1, 2); // Thursday
        let windows = week_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, until);
    }

    #[test]
    fn week_windows_since_is_monday() {
        // 2025-01-06 is a Monday.
        let since = d(2025, 1, 6);
        assert_eq!(since.weekday(), Weekday::Mon);
        let until = d(2025, 1, 13); // next Monday
        let windows = week_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, until);
    }

    #[test]
    fn week_windows_since_is_sunday_first_window_is_one_day() {
        // 2025-01-05 is Sunday; next Monday is 2025-01-06.
        let since = d(2025, 1, 5);
        assert_eq!(since.weekday(), Weekday::Sun);
        let until = d(2025, 1, 20);
        let windows = week_windows(since, until);
        assert!(windows.len() >= 2);
        assert_eq!(windows[0].since, since);
        assert_eq!(windows[0].until, d(2025, 1, 6));
        assert_eq!(window_len_days(&windows[0]), 1);
        // Subsequent windows begin on Monday.
        for w in &windows[1..] {
            assert_eq!(
                w.since.weekday(),
                Weekday::Mon,
                "expected Monday: {:?}",
                w.since
            );
        }
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn week_windows_multi_week_within_month() {
        // Wed 2025-01-01 .. Wed 2025-01-22 → segments cut on Mondays 01-06, 01-13, 01-20.
        let since = d(2025, 1, 1);
        let until = d(2025, 1, 22);
        let windows = week_windows(since, until);
        assert_eq!(windows.len(), 4);
        assert_eq!(
            windows[0],
            TimeWindow {
                since: d(2025, 1, 1),
                until: d(2025, 1, 6)
            }
        );
        assert_eq!(
            windows[1],
            TimeWindow {
                since: d(2025, 1, 6),
                until: d(2025, 1, 13)
            }
        );
        assert_eq!(
            windows[2],
            TimeWindow {
                since: d(2025, 1, 13),
                until: d(2025, 1, 20)
            }
        );
        assert_eq!(
            windows[3],
            TimeWindow {
                since: d(2025, 1, 20),
                until: d(2025, 1, 22)
            }
        );
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn week_windows_crosses_year_boundary() {
        // 2024-12-30 is a Monday; 2025-01-06 is a Monday.
        let since = d(2024, 12, 30);
        assert_eq!(since.weekday(), Weekday::Mon);
        let until = d(2025, 1, 8);
        let windows = week_windows(since, until);
        assert_eq!(windows.len(), 2);
        assert_eq!(
            windows[0],
            TimeWindow {
                since: d(2024, 12, 30),
                until: d(2025, 1, 6)
            }
        );
        assert_eq!(
            windows[1],
            TimeWindow {
                since: d(2025, 1, 6),
                until: d(2025, 1, 8)
            }
        );
        assert_contiguous(&windows, since, until);
    }

    // ---- day_windows ----

    #[test]
    fn day_windows_since_equals_until_is_empty() {
        let day = d(2025, 1, 1);
        assert!(day_windows(day, day).is_empty());
    }

    #[test]
    fn day_windows_since_after_until_is_empty() {
        assert!(day_windows(d(2025, 1, 2), d(2025, 1, 1)).is_empty());
    }

    #[test]
    fn day_windows_single_day() {
        let since = d(2025, 1, 1);
        let until = d(2025, 1, 2);
        let windows = day_windows(since, until);
        assert_eq!(windows.len(), 1);
        assert_eq!(windows[0], TimeWindow { since, until });
        assert_eq!(window_len_days(&windows[0]), 1);
    }

    #[test]
    fn day_windows_three_days() {
        let since = d(2025, 1, 1);
        let until = d(2025, 1, 4);
        let windows = day_windows(since, until);
        assert_eq!(windows.len(), 3);
        assert_eq!(
            windows[0],
            TimeWindow {
                since: d(2025, 1, 1),
                until: d(2025, 1, 2)
            }
        );
        assert_eq!(
            windows[1],
            TimeWindow {
                since: d(2025, 1, 2),
                until: d(2025, 1, 3)
            }
        );
        assert_eq!(
            windows[2],
            TimeWindow {
                since: d(2025, 1, 3),
                until: d(2025, 1, 4)
            }
        );
        assert_contiguous(&windows, since, until);
    }

    #[test]
    fn day_windows_spans_multiple_months_with_correct_count() {
        // Jan (31) + Feb 1..15 (14) = 45 days in [2025-01-01, 2025-02-15).
        let since = d(2025, 1, 1);
        let until = d(2025, 2, 15);
        let windows = day_windows(since, until);
        assert_eq!(windows.len(), 45);
        for w in &windows {
            assert_eq!(window_len_days(w), 1);
        }
        assert_contiguous(&windows, since, until);
    }

    // ---- window_len_days ----

    #[test]
    fn window_len_days_zero_when_since_equals_until() {
        let day = d(2025, 1, 1);
        let w = TimeWindow {
            since: day,
            until: day,
        };
        assert_eq!(window_len_days(&w), 0);
    }

    #[test]
    fn window_len_days_one_day() {
        let w = TimeWindow {
            since: d(2025, 1, 1),
            until: d(2025, 1, 2),
        };
        assert_eq!(window_len_days(&w), 1);
    }

    #[test]
    fn window_len_days_january_is_thirty_one() {
        let w = TimeWindow {
            since: d(2025, 1, 1),
            until: d(2025, 2, 1),
        };
        assert_eq!(window_len_days(&w), 31);
    }

    #[test]
    fn window_len_days_full_non_leap_year_is_365() {
        let w = TimeWindow {
            since: d(2025, 1, 1),
            until: d(2026, 1, 1),
        };
        assert_eq!(window_len_days(&w), 365);
    }

    #[test]
    fn window_len_days_full_leap_year_is_366() {
        let w = TimeWindow {
            since: d(2024, 1, 1),
            until: d(2025, 1, 1),
        };
        assert_eq!(window_len_days(&w), 366);
    }
}
