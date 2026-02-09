use chrono::{Datelike, NaiveDate, Weekday};
use shiplog_schema::coverage::TimeWindow;

/// Split a window into month-sized windows.
///
/// `until` is exclusive.
pub fn month_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;

    while cursor < until {
        let next = next_month_start(cursor);
        let end = if next < until { next } else { until };
        out.push(TimeWindow {
            since: cursor,
            until: end,
        });
        cursor = end;
    }

    out
}

/// Split a window into week-sized windows (Mon..Mon by default).
pub fn week_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;

    while cursor < until {
        let next = next_week_start(cursor, Weekday::Mon);
        let end = if next < until { next } else { until };
        out.push(TimeWindow {
            since: cursor,
            until: end,
        });
        cursor = end;
    }

    out
}

/// Split a window into day windows.
pub fn day_windows(since: NaiveDate, until: NaiveDate) -> Vec<TimeWindow> {
    if since >= until {
        return vec![];
    }

    let mut out = Vec::new();
    let mut cursor = since;
    while cursor < until {
        let end = cursor.succ_opt().unwrap_or(until);
        out.push(TimeWindow {
            since: cursor,
            until: if end < until { end } else { until },
        });
        cursor = end;
    }
    out
}

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
}
