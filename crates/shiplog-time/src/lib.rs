//! Time utilities and helpers for shiplog.
//!
//! This crate provides time-related utilities including formatting,
//! parsing, duration helpers, and time range types.

use chrono::{DateTime, Datelike, Duration, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};

/// A time range with start and end timestamps.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

impl TimeRange {
    /// Create a new time range.
    pub fn new(start: DateTime<Utc>, end: DateTime<Utc>) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    /// Check if a timestamp falls within the range.
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }

    /// Get the duration of the range.
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    /// Check if this range overlaps with another.
    pub fn overlaps(&self, other: &TimeRange) -> bool {
        self.start <= other.end && self.end >= other.start
    }
}

/// Duration helper for common time units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DurationHelper(pub Duration);

impl DurationHelper {
    /// Create from seconds.
    pub fn seconds(s: i64) -> Self {
        Self(Duration::seconds(s))
    }

    /// Create from minutes.
    pub fn minutes(m: i64) -> Self {
        Self(Duration::minutes(m))
    }

    /// Create from hours.
    pub fn hours(h: i64) -> Self {
        Self(Duration::hours(h))
    }

    /// Create from days.
    pub fn days(d: i64) -> Self {
        Self(Duration::days(d))
    }

    /// Create from weeks.
    pub fn weeks(w: i64) -> Self {
        Self(Duration::weeks(w))
    }

    /// Get the inner duration.
    pub fn as_duration(&self) -> Duration {
        self.0
    }
}

impl From<DurationHelper> for Duration {
    fn from(val: DurationHelper) -> Self {
        val.0
    }
}

/// Time period enum for common periods.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimePeriod {
    Today,
    Yesterday,
    ThisWeek,
    ThisMonth,
    ThisQuarter,
    ThisYear,
    Last7Days,
    Last30Days,
    Last90Days,
}

impl TimePeriod {
    /// Get the time range for this period.
    pub fn to_range(&self) -> Option<TimeRange> {
        let now = Utc::now();
        let today = now.date_naive();

        match self {
            TimePeriod::Today => {
                let start = today.and_hms_opt(0, 0, 0)?;
                let end = today.and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::Yesterday => {
                let yesterday = today - Duration::days(1);
                let start = yesterday.and_hms_opt(0, 0, 0)?;
                let end = yesterday.and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::ThisWeek => {
                let weekday = today.weekday();
                let days_since_monday = weekday.num_days_from_monday();
                let monday = today - Duration::days(days_since_monday as i64);
                let start = monday.and_hms_opt(0, 0, 0)?;
                let end = (monday + Duration::days(6)).and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::ThisMonth => {
                let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)?
                    .and_hms_opt(0, 0, 0)?;
                let end_month = if today.month() == 12 {
                    NaiveDate::from_ymd_opt(today.year() + 1, 1, 1)
                } else {
                    NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1)
                }?;
                let end = (end_month - Duration::days(1)).and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::ThisQuarter => {
                let quarter_start = ((today.month() - 1) / 3) * 3 + 1;
                let start = NaiveDate::from_ymd_opt(today.year(), quarter_start, 1)?
                    .and_hms_opt(0, 0, 0)?;
                let quarter_end = if quarter_start + 2 > 12 {
                    NaiveDate::from_ymd_opt(today.year() + 1, quarter_start + 2 - 12, 1)
                } else {
                    NaiveDate::from_ymd_opt(today.year(), quarter_start + 3, 1)
                }?;
                let end = (quarter_end - Duration::days(1)).and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::ThisYear => {
                let start = NaiveDate::from_ymd_opt(today.year(), 1, 1)?.and_hms_opt(0, 0, 0)?;
                let end = NaiveDate::from_ymd_opt(today.year(), 12, 31)?.and_hms_opt(23, 59, 59)?;
                Some(TimeRange {
                    start: DateTime::from_naive_utc_and_offset(start, Utc),
                    end: DateTime::from_naive_utc_and_offset(end, Utc),
                })
            }
            TimePeriod::Last7Days => {
                let end = now;
                let start = end - Duration::days(7);
                Some(TimeRange { start, end })
            }
            TimePeriod::Last30Days => {
                let end = now;
                let start = end - Duration::days(30);
                Some(TimeRange { start, end })
            }
            TimePeriod::Last90Days => {
                let end = now;
                let start = end - Duration::days(90);
                Some(TimeRange { start, end })
            }
        }
    }
}

/// Format a timestamp for display.
pub fn format_timestamp(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format a timestamp in ISO 8601 format.
pub fn format_iso8601(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Parse a timestamp from ISO 8601 format.
pub fn parse_iso8601(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

/// Get the start of the day for a given date.
pub fn start_of_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive()
        .and_hms_opt(0, 0, 0)
        .map(|d| DateTime::from_naive_utc_and_offset(d, Utc))
        .unwrap_or(dt)
}

/// Get the end of the day for a given date.
pub fn end_of_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    dt.date_naive()
        .and_hms_opt(23, 59, 59)
        .map(|d| DateTime::from_naive_utc_and_offset(d, Utc))
        .unwrap_or(dt)
}

/// Check if a date is a weekday.
pub fn is_weekday(dt: DateTime<Utc>) -> bool {
    !matches!(dt.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Check if a date is a weekend.
pub fn is_weekend(dt: DateTime<Utc>) -> bool {
    matches!(dt.weekday(), Weekday::Sat | Weekday::Sun)
}

/// Get relative time string (e.g., "2 hours ago").
pub fn relative_time(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now - dt;

    let secs = diff.num_seconds();

    if secs < 60 {
        format!("{} seconds ago", secs)
    } else if secs < 3600 {
        let mins = diff.num_minutes();
        format!("{} minute{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if secs < 86400 {
        let hours = diff.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if secs < 604800 {
        let days = diff.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else {
        format_timestamp(dt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_time_range_creation() {
        let start = Utc::now();
        let end = start + Duration::hours(1);

        let range = TimeRange::new(start, end);
        assert!(range.is_some());

        // Invalid range (start > end)
        let invalid = TimeRange::new(end, start);
        assert!(invalid.is_none());
    }

    #[test]
    fn test_time_range_contains() {
        let start = Utc::now();
        let end = start + Duration::hours(1);
        let range = TimeRange::new(start, end).unwrap();

        // Within range
        let middle = start + Duration::minutes(30);
        assert!(range.contains(middle));

        // Outside range
        let after = end + Duration::hours(1);
        assert!(!range.contains(after));
    }

    #[test]
    fn test_time_range_overlaps() {
        let start1 = Utc::now();
        let end1 = start1 + Duration::hours(2);

        let start2 = start1 + Duration::hours(1);
        let end2 = start2 + Duration::hours(2);

        let range1 = TimeRange::new(start1, end1).unwrap();
        let range2 = TimeRange::new(start2, end2).unwrap();

        assert!(range1.overlaps(&range2));
    }

    #[test]
    fn test_duration_helpers() {
        assert_eq!(
            DurationHelper::seconds(60).as_duration(),
            Duration::seconds(60)
        );
        assert_eq!(
            DurationHelper::minutes(5).as_duration(),
            Duration::minutes(5)
        );
        assert_eq!(DurationHelper::hours(2).as_duration(), Duration::hours(2));
        assert_eq!(DurationHelper::days(1).as_duration(), Duration::days(1));
        assert_eq!(DurationHelper::weeks(1).as_duration(), Duration::weeks(1));
    }

    #[test]
    fn test_time_period_today() {
        let period = TimePeriod::Today;
        let range = period.to_range();
        assert!(range.is_some());

        let r = range.unwrap();
        assert!(r.contains(Utc::now()));
    }

    #[test]
    fn test_time_period_last_7_days() {
        let period = TimePeriod::Last7Days;
        let range = period.to_range().unwrap();

        let now = Utc::now();
        let _seven_days_ago = now - Duration::days(7);

        // Check range spans roughly 7 days
        assert!(range.duration() >= Duration::days(6));
        assert!(range.duration() <= Duration::days(8));

        // Check that old date is not in range
        assert!(!range.contains(now - Duration::days(10)));
    }

    #[test]
    fn test_format_functions() {
        let dt = DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
            .unwrap()
            .with_timezone(&Utc);

        let formatted = format_timestamp(dt);
        assert!(formatted.contains("2024-01-15"));

        let iso = format_iso8601(dt);
        assert!(iso.contains("2024-01-15"));

        let parsed = parse_iso8601(&iso);
        assert!(parsed.is_some());
    }

    #[test]
    fn test_start_end_of_day() {
        let dt = Utc::now();

        let start = start_of_day(dt);
        assert_eq!(start.hour(), 0);
        assert_eq!(start.minute(), 0);
        assert_eq!(start.second(), 0);

        let end = end_of_day(dt);
        assert_eq!(end.hour(), 23);
        assert_eq!(end.minute(), 59);
        assert_eq!(end.second(), 59);
    }

    #[test]
    fn test_weekday_weekend() {
        // Create a known Saturday
        let sat = NaiveDate::from_ymd_opt(2024, 1, 6)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let sat_dt = DateTime::from_naive_utc_and_offset(sat, Utc);

        assert!(is_weekend(sat_dt));
        assert!(!is_weekday(sat_dt));

        // Create a known Monday
        let mon = NaiveDate::from_ymd_opt(2024, 1, 8)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let mon_dt = DateTime::from_naive_utc_and_offset(mon, Utc);

        assert!(is_weekday(mon_dt));
        assert!(!is_weekend(mon_dt));
    }
}
