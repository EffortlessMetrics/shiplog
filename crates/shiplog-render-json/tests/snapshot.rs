//! Snapshot tests for shiplog-render-json output formats.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::RunId;
use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_testkit::pr_event;

// ── JSONL snapshot tests ────────────────────────────────────────────────

#[test]
fn snapshot_single_event_jsonl() {
    let events = vec![pr_event("acme/widgets", 42, "Add caching layer")];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("single_event_jsonl", text);
}

#[test]
fn snapshot_multiple_events_jsonl() {
    let events = vec![
        pr_event("acme/widgets", 1, "Bootstrap project"),
        pr_event("acme/widgets", 2, "Add CI pipeline"),
        pr_event("acme/widgets", 3, "Implement auth flow"),
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("multiple_events_jsonl", text);
}

#[test]
fn snapshot_empty_events_jsonl() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &[]).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("empty_events_jsonl", text);
}

#[test]
fn snapshot_special_characters_in_title() {
    let events = vec![
        pr_event("acme/widgets", 10, "Fix \"quoted\" title & <html> escapes"),
        pr_event("acme/widgets", 11, "Handle unicode: 日本語テスト 🚀"),
        pr_event("acme/widgets", 12, "Newline in title\\nshould be escaped"),
    ];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    write_events_jsonl(&path, &events).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("special_characters_jsonl", text);
}

// ── Coverage manifest snapshot tests ────────────────────────────────────

fn deterministic_coverage(completeness: Completeness) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("snapshot_run_2025".into()),
        generated_at: Utc.timestamp_opt(1_704_067_200, 0).unwrap(), // 2024-01-01T00:00:00Z
        user: "testuser".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness,
    }
}

#[test]
fn snapshot_coverage_manifest_complete() {
    let cov = deterministic_coverage(Completeness::Complete);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_complete", text);
}

#[test]
fn snapshot_coverage_manifest_partial_with_slices() {
    let mut cov = deterministic_coverage(Completeness::Partial);
    cov.slices = vec![CoverageSlice {
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        query: "author:testuser is:merged".into(),
        total_count: 500,
        fetched: 200,
        incomplete_results: Some(true),
        notes: vec!["API rate limit reached".into()],
    }];
    cov.warnings = vec!["Incomplete data for January 2025".into()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_partial", text);
}

#[test]
fn snapshot_coverage_manifest_multiple_sources() {
    let mut cov = deterministic_coverage(Completeness::Complete);
    cov.sources = vec!["github".into(), "manual".into()];

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&path, &cov).unwrap();

    let text = std::fs::read_to_string(&path).unwrap();
    insta::assert_snapshot!("coverage_manifest_multiple_sources", text);
}
