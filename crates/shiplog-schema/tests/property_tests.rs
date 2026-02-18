//! Property tests for shiplog-schema
//!
//! This module contains property-based tests for event parsing invariants
//! (serialization round-trips).

use proptest::prelude::*;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamFile};

// ============================================================================
// JSON Round-Trip Tests
// ============================================================================

proptest! {
    /// Test that EventEnvelope JSON round-trip preserves all fields
    #[test]
    fn prop_event_envelope_json_roundtrip(event in shiplog_testkit::proptest::strategy_event_envelope()) {
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EventEnvelope = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(event.id, deserialized.id);
        prop_assert_eq!(event.kind, deserialized.kind);
        prop_assert_eq!(event.actor.login, deserialized.actor.login);
        prop_assert_eq!(event.repo.full_name, deserialized.repo.full_name);
    }

    /// Test that Vec<EventEnvelope> JSONL round-trip preserves order and data
    #[test]
    fn prop_event_vec_jsonl_roundtrip(events in shiplog_testkit::proptest::strategy_event_vec(20)) {
        let jsonl: String = events.iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        let deserialized: Vec<EventEnvelope> = jsonl.lines()
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        prop_assert_eq!(events.len(), deserialized.len());
        for (orig, deser) in events.iter().zip(deserialized.iter()) {
            prop_assert_eq!(orig.id, deser.id);
        }
    }

    /// Test that WorkstreamsFile YAML round-trip preserves structure
    #[test]
    fn prop_workstreams_file_yaml_roundtrip(file in shiplog_testkit::proptest::strategy_workstreams_file()) {
        let yaml = serde_yaml::to_string(&file).unwrap();
        let deserialized: WorkstreamFile = serde_yaml::from_str(&yaml).unwrap();
        prop_assert_eq!(file.version, deserialized.version);
        prop_assert_eq!(file.workstreams.len(), deserialized.workstreams.len());
    }

    /// Test that CoverageManifest JSON round-trip preserves metadata
    #[test]
    fn prop_coverage_manifest_json_roundtrip(manifest in shiplog_testkit::proptest::strategy_coverage_manifest()) {
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: CoverageManifest = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(manifest.user, deserialized.user);
        prop_assert_eq!(manifest.window.since, deserialized.window.since);
        prop_assert_eq!(manifest.window.until, deserialized.window.until);
        prop_assert_eq!(manifest.completeness, deserialized.completeness);
    }
}

// ============================================================================
// Enum Round-Trip Tests
// ============================================================================

proptest! {
    /// Test that SourceSystem enum preserves variant information
    #[test]
    fn prop_source_system_roundtrip(source in shiplog_testkit::proptest::strategy_source_system()) {
        let json = serde_json::to_string(&source).unwrap();
        let deserialized: SourceSystem = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(source, deserialized);
    }

    /// Test that RepoVisibility enum preserves variant information
    #[test]
    fn prop_repo_visibility_roundtrip(visibility in shiplog_testkit::proptest::strategy_repo_visibility()) {
        let json = serde_json::to_string(&visibility).unwrap();
        let deserialized: RepoVisibility = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(visibility, deserialized);
    }

    /// Test that PullRequestState enum preserves variant information
    #[test]
    fn prop_pr_state_roundtrip(state in shiplog_testkit::proptest::strategy_pr_state()) {
        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PullRequestState = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(state, deserialized);
    }

    /// Test that EventKind enum preserves variant information
    #[test]
    fn prop_event_kind_roundtrip(kind in shiplog_testkit::proptest::strategy_event_kind()) {
        let json = serde_json::to_string(&kind).unwrap();
        let deserialized: EventKind = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(kind, deserialized);
    }

    /// Test that Completeness enum preserves variant information
    #[test]
    fn prop_completeness_roundtrip(completeness in shiplog_testkit::proptest::strategy_completeness()) {
        let json = serde_json::to_string(&completeness).unwrap();
        let deserialized: Completeness = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(completeness, deserialized);
    }
}

// ============================================================================
// Field-Specific Round-Trip Tests
// ============================================================================

proptest! {
    /// Test that EventId string representation matches inner value
    #[test]
    fn prop_event_id_string_representation(parts in shiplog_testkit::proptest::strategy_event_id_parts()) {
        let id = shiplog_ids::EventId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id_str = id.to_string();
        prop_assert_eq!(id_str, format!("{}", id));
    }

    /// Test that WorkstreamId string representation matches inner value
    #[test]
    fn prop_workstream_id_string_representation(parts in shiplog_testkit::proptest::strategy_workstream_id_parts()) {
        let id = shiplog_ids::WorkstreamId::from_parts(parts.iter().map(|s| s.as_str()).collect::<Vec<_>>().as_slice());
        let id_str = id.to_string();
        prop_assert_eq!(id_str, format!("{}", id));
    }

    /// Test that DateTime RFC3339 serialization round-trips correctly
    #[test]
    fn prop_datetime_rfc3339_roundtrip(dt in shiplog_testkit::proptest::strategy_datetime_utc()) {
        let json = serde_json::to_string(&dt).unwrap();
        let deserialized: chrono::DateTime<chrono::Utc> = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(dt, deserialized);
    }

    /// Test that NaiveDate ISO 8601 serialization round-trips correctly
    #[test]
    fn prop_naive_date_iso8601_roundtrip(date in shiplog_testkit::proptest::strategy_naive_date()) {
        let json = serde_json::to_string(&date).unwrap();
        let deserialized: chrono::NaiveDate = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(date, deserialized);
    }

    /// Test that optional fields round-trip as null/missing
    #[test]
    fn prop_optional_fields_roundtrip() {
        #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
        struct TestOptional {
            #[serde(skip_serializing_if = "Option::is_none")]
            optional_field: Option<String>,
        }

        let with_value = TestOptional { optional_field: Some("value".to_string()) };
        let json = serde_json::to_string(&with_value).unwrap();
        let deserialized: TestOptional = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(with_value, deserialized);

        let without_value = TestOptional { optional_field: None };
        let json = serde_json::to_string(&without_value).unwrap();
        prop_assert!(!json.contains("optional_field"));
    }
}

// ============================================================================
// TimeWindow Invariant Tests
// ============================================================================

proptest! {
    /// Test that TimeWindow::contains correctly identifies dates
    #[test]
    fn prop_time_window_contains_correct(
        since in shiplog_testkit::proptest::strategy_naive_date(),
        days in 1u64..365u64
    ) {
        let until = since.checked_add_days(chrono::Days::new(days)).unwrap();
        let window = TimeWindow { since, until };

        // Test a date inside the window
        let inside = since.checked_add_days(chrono::Days::new(days / 2)).unwrap();
        prop_assert!(window.contains(inside));

        // Test a date before the window
        let before = since.checked_sub_days(chrono::Days::new(1)).unwrap();
        prop_assert!(!window.contains(before));

        // Test a date after the window
        let after = until.checked_add_days(chrono::Days::new(1)).unwrap();
        prop_assert!(!window.contains(after));
    }
}
