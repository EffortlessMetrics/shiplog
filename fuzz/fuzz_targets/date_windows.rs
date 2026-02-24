#![no_main]

use chrono::NaiveDate;
use libfuzzer_sys::fuzz_target;
use shiplog_date_windows::{day_windows, month_windows, week_windows};

fn check_partition(windows: &[shiplog_schema::coverage::TimeWindow], since: NaiveDate, until: NaiveDate) {
    if windows.is_empty() {
        assert!(since >= until);
        return;
    }

    assert_eq!(windows.first().unwrap().since, since);
    assert_eq!(windows.last().unwrap().until, until);

    for (idx, window) in windows.iter().enumerate() {
        assert!(window.since < window.until);

        if idx + 1 < windows.len() {
            assert_eq!(window.until, windows[idx + 1].since);
        }
    }

    let total = windows
        .iter()
        .map(|window| (window.until - window.since).num_days())
        .sum::<i64>();
    assert_eq!(total, (until - since).num_days());
}

fuzz_target!(|data: &[u8]| {
    let mut padded = [0u8; 24];
    let copy_len = data.len().min(padded.len());
    padded[..copy_len].copy_from_slice(&data[..copy_len]);

    let since_raw = i64::from_le_bytes(padded[0..8].try_into().unwrap());
    let until_raw = i64::from_le_bytes(padded[8..16].try_into().unwrap());

    let clamp = 20_000i64;
    let since_offset = (since_raw % clamp).abs() as i32;
    let until_offset = (until_raw % clamp).abs() as i32;

    let since0 = NaiveDate::from_num_days_from_ce_opt(7_000_000 + since_offset).unwrap();
    let until0 = NaiveDate::from_num_days_from_ce_opt(7_000_000 + until_offset).unwrap();

    let (since, until) = if padded[16] & 1 == 1 {
        (until0, since0)
    } else {
        (since0, until0)
    };

    let by_month = month_windows(since, until);
    let by_week = week_windows(since, until);
    let by_day = day_windows(since, until);

    check_partition(&by_month, since, until);
    check_partition(&by_week, since, until);
    check_partition(&by_day, since, until);
});
