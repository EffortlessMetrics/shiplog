//! Integration tests for shiplog-time.

use chrono::{DateTime, Duration, NaiveDate, Timelike, Utc};
use proptest::prelude::*;
use shiplog_time::*;

// ── Known-answer tests ──────────────────────────────────────────

#[test]
fn format_timestamp_known() {
    let dt = DateTime::parse_from_rfc3339("2024-01-15T10:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    assert_eq!(format_timestamp(dt), "2024-01-15 10:30:45 UTC");
}

#[test]
fn format_iso8601_known() {
    let dt = DateTime::parse_from_rfc3339("2024-01-15T10:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    let iso = format_iso8601(dt);
    assert!(iso.starts_with("2024-01-15T10:30:45"));
}

#[test]
fn parse_iso8601_valid() {
    let dt = parse_iso8601("2024-01-15T10:30:45Z");
    assert!(dt.is_some());
    let dt = dt.unwrap();
    assert_eq!(
        dt.date_naive(),
        NaiveDate::from_ymd_opt(2024, 1, 15).unwrap()
    );
}

#[test]
fn parse_iso8601_with_offset() {
    let dt = parse_iso8601("2024-01-15T10:30:45+05:00");
    assert!(dt.is_some());
}

#[test]
fn parse_iso8601_invalid() {
    assert!(parse_iso8601("").is_none());
    assert!(parse_iso8601("not a date").is_none());
    assert!(parse_iso8601("2024-01-15").is_none());
}

// ── format/parse round-trip ─────────────────────────────────────

#[test]
fn iso8601_roundtrip() {
    let dt = DateTime::parse_from_rfc3339("2024-06-15T14:30:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let iso = format_iso8601(dt);
    let parsed = parse_iso8601(&iso).unwrap();
    assert_eq!(dt, parsed);
}

// ── TimeRange tests ─────────────────────────────────────────────

#[test]
fn time_range_valid() {
    let start = Utc::now();
    let end = start + Duration::hours(1);
    let range = TimeRange::new(start, end);
    assert!(range.is_some());
}

#[test]
fn time_range_invalid_reversed() {
    let start = Utc::now();
    let end = start - Duration::hours(1);
    assert!(TimeRange::new(start, end).is_none());
}

#[test]
fn time_range_same_instant() {
    let now = Utc::now();
    let range = TimeRange::new(now, now);
    assert!(range.is_some());
    assert!(range.unwrap().contains(now));
}

#[test]
fn time_range_contains_boundaries() {
    let start = Utc::now();
    let end = start + Duration::hours(1);
    let range = TimeRange::new(start, end).unwrap();
    assert!(range.contains(start));
    assert!(range.contains(end));
}

#[test]
fn time_range_does_not_contain_outside() {
    let start = Utc::now();
    let end = start + Duration::hours(1);
    let range = TimeRange::new(start, end).unwrap();
    assert!(!range.contains(start - Duration::seconds(1)));
    assert!(!range.contains(end + Duration::seconds(1)));
}

#[test]
fn time_range_duration() {
    let start = Utc::now();
    let end = start + Duration::hours(2);
    let range = TimeRange::new(start, end).unwrap();
    assert_eq!(range.duration(), Duration::hours(2));
}

#[test]
fn time_range_overlaps_partial() {
    let r1 = TimeRange::new(Utc::now(), Utc::now() + Duration::hours(2)).unwrap();
    let r2 = TimeRange::new(
        Utc::now() + Duration::hours(1),
        Utc::now() + Duration::hours(3),
    )
    .unwrap();
    assert!(r1.overlaps(&r2));
    assert!(r2.overlaps(&r1));
}

#[test]
fn time_range_no_overlap() {
    let now = Utc::now();
    let r1 = TimeRange::new(now, now + Duration::hours(1)).unwrap();
    let r2 = TimeRange::new(now + Duration::hours(2), now + Duration::hours(3)).unwrap();
    assert!(!r1.overlaps(&r2));
}

#[test]
fn time_range_touching_boundaries_overlap() {
    let now = Utc::now();
    let r1 = TimeRange::new(now, now + Duration::hours(1)).unwrap();
    let r2 = TimeRange::new(now + Duration::hours(1), now + Duration::hours(2)).unwrap();
    // Touching at a point should overlap (start <= other.end && end >= other.start)
    assert!(r1.overlaps(&r2));
}

// ── DurationHelper tests ────────────────────────────────────────

#[test]
fn duration_helper_conversions() {
    assert_eq!(
        DurationHelper::seconds(60).as_duration(),
        Duration::seconds(60)
    );
    assert_eq!(
        DurationHelper::minutes(5).as_duration(),
        Duration::minutes(5)
    );
    assert_eq!(DurationHelper::hours(2).as_duration(), Duration::hours(2));
    assert_eq!(DurationHelper::days(7).as_duration(), Duration::days(7));
    assert_eq!(DurationHelper::weeks(1).as_duration(), Duration::weeks(1));
}

#[test]
fn duration_helper_into_duration() {
    let d: Duration = DurationHelper::hours(3).into();
    assert_eq!(d, Duration::hours(3));
}

// ── start_of_day / end_of_day ───────────────────────────────────

#[test]
fn start_of_day_known() {
    let dt = DateTime::parse_from_rfc3339("2024-06-15T14:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    let start = start_of_day(dt);
    assert_eq!(
        start,
        DateTime::parse_from_rfc3339("2024-06-15T00:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    );
}

#[test]
fn end_of_day_known() {
    let dt = DateTime::parse_from_rfc3339("2024-06-15T14:30:45Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = end_of_day(dt);
    assert_eq!(
        end,
        DateTime::parse_from_rfc3339("2024-06-15T23:59:59Z")
            .unwrap()
            .with_timezone(&Utc)
    );
}

// ── is_weekday / is_weekend ─────────────────────────────────────

#[test]
fn weekday_weekend_known_dates() {
    // 2024-01-06 = Saturday
    let sat = NaiveDate::from_ymd_opt(2024, 1, 6)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let sat_dt = DateTime::from_naive_utc_and_offset(sat, Utc);
    assert!(is_weekend(sat_dt));
    assert!(!is_weekday(sat_dt));

    // 2024-01-07 = Sunday
    let sun = NaiveDate::from_ymd_opt(2024, 1, 7)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let sun_dt = DateTime::from_naive_utc_and_offset(sun, Utc);
    assert!(is_weekend(sun_dt));

    // 2024-01-08 = Monday
    let mon = NaiveDate::from_ymd_opt(2024, 1, 8)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let mon_dt = DateTime::from_naive_utc_and_offset(mon, Utc);
    assert!(is_weekday(mon_dt));
    assert!(!is_weekend(mon_dt));

    // 2024-01-12 = Friday
    let fri = NaiveDate::from_ymd_opt(2024, 1, 12)
        .unwrap()
        .and_hms_opt(12, 0, 0)
        .unwrap();
    let fri_dt = DateTime::from_naive_utc_and_offset(fri, Utc);
    assert!(is_weekday(fri_dt));
}

// ── TimePeriod tests ────────────────────────────────────────────

#[test]
fn time_period_today_contains_now() {
    let range = TimePeriod::Today.to_range().unwrap();
    assert!(range.contains(Utc::now()));
}

#[test]
fn time_period_yesterday_does_not_contain_now() {
    let range = TimePeriod::Yesterday.to_range().unwrap();
    // "now" should not be in yesterday's range (unless it's exactly midnight)
    // This test may be flaky at midnight; unlikely in practice
    let now = Utc::now();
    if now.time().hour() > 0 {
        assert!(!range.contains(now));
    }
}

#[test]
fn time_period_all_variants_produce_ranges() {
    let periods = [
        TimePeriod::Today,
        TimePeriod::Yesterday,
        TimePeriod::ThisWeek,
        TimePeriod::ThisMonth,
        TimePeriod::ThisQuarter,
        TimePeriod::ThisYear,
        TimePeriod::Last7Days,
        TimePeriod::Last30Days,
        TimePeriod::Last90Days,
    ];
    for period in &periods {
        assert!(
            period.to_range().is_some(),
            "{:?} should produce a range",
            period
        );
    }
}

#[test]
fn time_period_last_7_days_duration() {
    let range = TimePeriod::Last7Days.to_range().unwrap();
    let dur = range.duration();
    assert!(dur >= Duration::days(6));
    assert!(dur <= Duration::days(8));
}

#[test]
fn time_period_last_30_days_duration() {
    let range = TimePeriod::Last30Days.to_range().unwrap();
    let dur = range.duration();
    assert!(dur >= Duration::days(29));
    assert!(dur <= Duration::days(31));
}

// ── relative_time tests ─────────────────────────────────────────

#[test]
fn relative_time_seconds() {
    let dt = Utc::now() - Duration::seconds(30);
    let result = relative_time(dt);
    assert!(result.contains("seconds ago"));
}

#[test]
fn relative_time_minutes() {
    let dt = Utc::now() - Duration::minutes(5);
    let result = relative_time(dt);
    assert!(result.contains("minutes ago"));
}

#[test]
fn relative_time_hours() {
    let dt = Utc::now() - Duration::hours(3);
    let result = relative_time(dt);
    assert!(result.contains("hours ago"));
}

#[test]
fn relative_time_days() {
    let dt = Utc::now() - Duration::days(3);
    let result = relative_time(dt);
    assert!(result.contains("days ago"));
}

#[test]
fn relative_time_old_shows_timestamp() {
    let dt = Utc::now() - Duration::days(14);
    let result = relative_time(dt);
    // Falls through to format_timestamp for >7 days
    assert!(result.contains("UTC"));
}

// ── Serde tests ─────────────────────────────────────────────────

#[test]
fn time_range_serde_roundtrip() {
    let start = DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let end = DateTime::parse_from_rfc3339("2024-12-31T23:59:59Z")
        .unwrap()
        .with_timezone(&Utc);
    let range = TimeRange::new(start, end).unwrap();
    let json = serde_json::to_string(&range).unwrap();
    let deserialized: TimeRange = serde_json::from_str(&json).unwrap();
    assert_eq!(range, deserialized);
}

#[test]
fn time_period_serde_roundtrip() {
    let period = TimePeriod::Last30Days;
    let json = serde_json::to_string(&period).unwrap();
    let deserialized: TimePeriod = serde_json::from_str(&json).unwrap();
    assert_eq!(period, deserialized);
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_start_of_day_is_midnight(
        year in 2000i32..2100,
        month in 1u32..=12,
        day in 1u32..=28,
    ) {
        let naive = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(12, 30, 45)
            .unwrap();
        let dt = DateTime::from_naive_utc_and_offset(naive, Utc);
        let start = start_of_day(dt);
        use chrono::Timelike;
        prop_assert_eq!(start.hour(), 0);
        prop_assert_eq!(start.minute(), 0);
        prop_assert_eq!(start.second(), 0);
    }

    #[test]
    fn prop_end_of_day_is_2359(
        year in 2000i32..2100,
        month in 1u32..=12,
        day in 1u32..=28,
    ) {
        let naive = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(12, 30, 45)
            .unwrap();
        let dt = DateTime::from_naive_utc_and_offset(naive, Utc);
        let end = end_of_day(dt);
        use chrono::Timelike;
        prop_assert_eq!(end.hour(), 23);
        prop_assert_eq!(end.minute(), 59);
        prop_assert_eq!(end.second(), 59);
    }

    #[test]
    fn prop_weekday_xor_weekend(
        year in 2000i32..2100,
        month in 1u32..=12,
        day in 1u32..=28,
    ) {
        let naive = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap();
        let dt = DateTime::from_naive_utc_and_offset(naive, Utc);
        prop_assert_eq!(is_weekday(dt), !is_weekend(dt));
    }

    #[test]
    fn prop_time_range_duration_non_negative(
        offset_hours in 0i64..1000,
    ) {
        let start = Utc::now();
        let end = start + Duration::hours(offset_hours);
        let range = TimeRange::new(start, end).unwrap();
        prop_assert!(range.duration().num_seconds() >= 0);
    }

    #[test]
    fn prop_iso8601_roundtrip(
        year in 2000i32..2100,
        month in 1u32..=12,
        day in 1u32..=28,
        hour in 0u32..24,
        min in 0u32..60,
        sec in 0u32..60,
    ) {
        let naive = NaiveDate::from_ymd_opt(year, month, day)
            .unwrap()
            .and_hms_opt(hour, min, sec)
            .unwrap();
        let dt = DateTime::from_naive_utc_and_offset(naive, Utc);
        let iso = format_iso8601(dt);
        let parsed = parse_iso8601(&iso).unwrap();
        prop_assert_eq!(dt, parsed);
    }
}
