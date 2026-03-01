//! Snapshot tests for coverage manifest serialization format.
//!
//! Uses insta to lock down the JSON structure of coverage types,
//! ensuring format stability across changes.

use chrono::NaiveDate;
use shiplog_coverage::{month_windows, window_len_days};
use shiplog_schema::coverage::{CoverageSlice, TimeWindow};

#[test]
fn snapshot_time_window_json() {
    let window = TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    };

    insta::assert_json_snapshot!("time_window", window);
}

#[test]
fn snapshot_complete_coverage_slice() {
    let slice = CoverageSlice {
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        query: "github prs merged:2025-01-01..2025-02-01".to_string(),
        total_count: 15,
        fetched: 15,
        incomplete_results: Some(false),
        notes: vec![],
    };

    insta::assert_json_snapshot!("complete_coverage_slice", slice);
}

#[test]
fn snapshot_incomplete_coverage_slice() {
    let slice = CoverageSlice {
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        query: "github prs merged:2025-01-01..2025-02-01".to_string(),
        total_count: 50,
        fetched: 30,
        incomplete_results: Some(true),
        notes: vec![
            "GitHub search API limit reached".to_string(),
            "Consider using smaller date windows".to_string(),
        ],
    };

    insta::assert_json_snapshot!("incomplete_coverage_slice", slice);
}

#[test]
fn snapshot_quarter_month_windows() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    let summary: Vec<_> = windows
        .iter()
        .map(|w| {
            serde_json::json!({
                "since": w.since.to_string(),
                "until": w.until.to_string(),
                "days": window_len_days(w),
            })
        })
        .collect();

    insta::assert_json_snapshot!("quarter_month_windows", summary);
}

#[test]
fn snapshot_coverage_slices_with_mixed_status() {
    let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
    let windows = month_windows(since, until);

    let slices = vec![
        CoverageSlice {
            window: windows[0].clone(),
            query: "github prs jan".to_string(),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec![],
        },
        CoverageSlice {
            window: windows[1].clone(),
            query: "github prs feb".to_string(),
            total_count: 50,
            fetched: 30,
            incomplete_results: Some(true),
            notes: vec!["Rate limited".to_string()],
        },
        CoverageSlice {
            window: windows[2].clone(),
            query: "github prs mar".to_string(),
            total_count: 8,
            fetched: 8,
            incomplete_results: Some(false),
            notes: vec![],
        },
    ];

    insta::assert_json_snapshot!("mixed_status_slices", slices);
}
