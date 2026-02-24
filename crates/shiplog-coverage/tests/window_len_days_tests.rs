use chrono::NaiveDate;
use shiplog_coverage::window_len_days;
use shiplog_schema::coverage::TimeWindow;

#[test]
fn window_len_days_reports_multi_day_length() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 1, 4).unwrap(),
    };

    assert_eq!(window_len_days(&window), 3);
}
