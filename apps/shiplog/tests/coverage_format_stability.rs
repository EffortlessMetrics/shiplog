//! Format stability tests for shiplog-coverage.
//!
//! Complements `snapshot.rs` with full CoverageManifest snapshots, minimal
//! variants, and edge-case values.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::RunId;
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

// ============================================================================
// Full CoverageManifest
// ============================================================================

#[test]
fn coverage_manifest_full_json() {
    let manifest = CoverageManifest {
        run_id: RunId("stability_test_run".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into(), "manual".into()],
        slices: vec![
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "author:octocat is:merged merged:2025-01-01..2025-02-01".into(),
                total_count: 15,
                fetched: 15,
                incomplete_results: Some(false),
                notes: vec![],
            },
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                },
                query: "author:octocat is:merged merged:2025-02-01..2025-03-01".into(),
                total_count: 100,
                fetched: 30,
                incomplete_results: Some(true),
                notes: vec![
                    "GitHub search API limit reached".into(),
                    "Results may be incomplete".into(),
                ],
            },
        ],
        warnings: vec!["Incomplete results for February slice".into()],
        completeness: Completeness::Partial,
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn coverage_manifest_full_yaml() {
    let manifest = CoverageManifest {
        run_id: RunId("yaml_test_run".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            query: "author:octocat is:merged".into(),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec![],
        }],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    insta::assert_yaml_snapshot!(manifest);
}

// ============================================================================
// Minimal CoverageManifest
// ============================================================================

#[test]
fn coverage_manifest_minimal() {
    let manifest = CoverageManifest {
        run_id: RunId("min".into()),
        generated_at: ts(),
        user: "u".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 2).unwrap(),
        },
        mode: "created".into(),
        sources: vec![],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Unknown,
    };
    insta::assert_json_snapshot!(manifest);
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn coverage_manifest_edge_cases() {
    let manifest = CoverageManifest {
        run_id: RunId("".into()),
        generated_at: ts(),
        user: "user with spaces & <special>".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
        },
        mode: "".into(),
        sources: vec!["".into(), "github".into(), "日本語ソース".into()],
        slices: vec![CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2099, 12, 31).unwrap(),
            },
            query: "query with \"quotes\" & <brackets>".into(),
            total_count: u64::MAX,
            fetched: 0,
            incomplete_results: None,
            notes: vec!["".into(), "Special chars: <>&\"'".into()],
        }],
        warnings: vec![
            "".into(),
            "Warning with émojis 🚀".into(),
            "Multi\nline\nwarning".into(),
        ],
        completeness: Completeness::Partial,
    };
    insta::assert_json_snapshot!(manifest);
}

#[test]
fn coverage_slice_no_incomplete_results() {
    let slice = CoverageSlice {
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
        },
        query: "test".into(),
        total_count: 0,
        fetched: 0,
        incomplete_results: None,
        notes: vec![],
    };
    insta::assert_json_snapshot!(slice);
}
