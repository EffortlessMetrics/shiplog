//! Property-based tests for shiplog-render-json.

use crate::artifact_json::{write_coverage_manifest, write_events_jsonl};
use proptest::prelude::*;
use shiplog::schema::coverage::CoverageManifest;
use shiplog::schema::event::EventEnvelope;
use shiplog_testkit::proptest::{
    strategy_coverage_manifest, strategy_event_envelope, strategy_event_vec,
};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// JSONL roundtrip: write then read back preserves event identity.
    #[test]
    fn prop_jsonl_roundtrip(events in strategy_event_vec(30)) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        if events.is_empty() {
            prop_assert!(text.is_empty());
        } else {
            let lines: Vec<&str> = text.lines().collect();
            prop_assert_eq!(lines.len(), events.len());
            for (i, line) in lines.iter().enumerate() {
                let parsed: EventEnvelope = serde_json::from_str(line)
                    .unwrap_or_else(|e| panic!("line {} failed: {}", i, e));
                prop_assert_eq!(&parsed.id, &events[i].id);
                prop_assert_eq!(&parsed.kind, &events[i].kind);
            }
        }
    }

    /// Coverage manifest roundtrip: write then read back preserves fields.
    #[test]
    fn prop_coverage_manifest_roundtrip(cov in strategy_coverage_manifest()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("coverage.json");

        write_coverage_manifest(&path, &cov).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();

        prop_assert_eq!(&loaded.user, &cov.user);
        prop_assert_eq!(&loaded.mode, &cov.mode);
        prop_assert_eq!(&loaded.window, &cov.window);
        prop_assert_eq!(&loaded.completeness, &cov.completeness);
        prop_assert_eq!(loaded.slices.len(), cov.slices.len());
    }

    /// JSONL output is deterministic: same events produce identical output.
    #[test]
    fn prop_jsonl_output_is_deterministic(events in strategy_event_vec(20)) {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let path1 = dir1.path().join("events.jsonl");
        let path2 = dir2.path().join("events.jsonl");

        write_events_jsonl(&path1, &events).unwrap();
        write_events_jsonl(&path2, &events).unwrap();

        let text1 = std::fs::read_to_string(&path1).unwrap();
        let text2 = std::fs::read_to_string(&path2).unwrap();
        prop_assert_eq!(text1, text2, "JSONL output should be deterministic");
    }

    /// Line count in JSONL matches event count.
    #[test]
    fn prop_jsonl_line_count_matches_event_count(events in strategy_event_vec(50)) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        if events.is_empty() {
            prop_assert!(text.is_empty());
        } else {
            let line_count = text.lines().count();
            prop_assert_eq!(line_count, events.len());
        }
    }

    /// Single event roundtrip preserves key fields.
    #[test]
    fn prop_single_event_roundtrip(event in strategy_event_envelope()) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let events = [event];

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let parsed: EventEnvelope = serde_json::from_str(text.trim()).unwrap();
        prop_assert_eq!(&parsed.id, &events[0].id);
        prop_assert_eq!(&parsed.kind, &events[0].kind);
        prop_assert_eq!(&parsed.actor.login, &events[0].actor.login);
        prop_assert_eq!(&parsed.repo.full_name, &events[0].repo.full_name);
    }

    /// Every line in JSONL output is valid JSON.
    #[test]
    fn prop_jsonl_each_line_is_valid_json(events in strategy_event_vec(30)) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        for (i, line) in text.lines().enumerate() {
            let parsed: serde_json::Value = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {} is not valid JSON: {}", i, e));
            prop_assert!(parsed.is_object(), "line {} should be a JSON object", i);
        }
    }
}
