//! Property-based tests for the shiplog-engine orchestration pipeline.

use proptest::prelude::*;
use shiplog_engine::Engine;
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_testkit::proptest::strategies::{strategy_coverage_manifest, strategy_event_vec};
use shiplog_workstream_cluster::RepoClusterer;

fn engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"prop-test-key")));
    Engine::new(renderer, clusterer, redactor)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    /// The engine should never panic, regardless of input events.
    #[test]
    fn run_never_panics(
        events in strategy_event_vec(20),
        coverage in strategy_coverage_manifest(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("prop_run");
        let engine = engine();
        let ingest = IngestOutput { events, coverage };
        // We only care that it doesn't panic; errors are acceptable.
        let _ = engine.run(
            ingest,
            "proptester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        );
    }

    /// All canonical output files should exist when run() succeeds.
    #[test]
    fn run_success_implies_all_artifacts_exist(
        events in strategy_event_vec(10),
        coverage in strategy_coverage_manifest(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("prop_artifacts");
        let engine = engine();
        let ingest = IngestOutput { events, coverage };

        if let Ok((outputs, _)) = engine.run(
            ingest,
            "proptester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        ) {
            prop_assert!(outputs.packet_md.exists());
            prop_assert!(outputs.ledger_events_jsonl.exists());
            prop_assert!(outputs.coverage_manifest_json.exists());
            prop_assert!(outputs.bundle_manifest_json.exists());
            prop_assert!(outputs.zip_path.is_none());
        }
    }

    /// The number of lines in the ledger must equal the number of input events.
    #[test]
    fn ledger_event_count_matches_input(
        events in strategy_event_vec(15),
        coverage in strategy_coverage_manifest(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("prop_ledger");
        let engine = engine();
        let event_count = events.len();
        let ingest = IngestOutput { events, coverage };

        if let Ok((outputs, _)) = engine.run(
            ingest,
            "proptester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        ) {
            let content = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
            let lines = content.lines().count();
            prop_assert_eq!(lines, event_count);
        }
    }

    /// The coverage manifest should always be valid JSON containing the user.
    #[test]
    fn coverage_manifest_valid_json_with_user(
        events in strategy_event_vec(5),
        coverage in strategy_coverage_manifest(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("prop_coverage");
        let engine = engine();
        let expected_user = coverage.user.clone();
        let ingest = IngestOutput { events, coverage };

        if let Ok((outputs, _)) = engine.run(
            ingest,
            &expected_user,
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        ) {
            let json_str = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();
            let parsed: CoverageManifest = serde_json::from_str(&json_str).unwrap();
            prop_assert_eq!(parsed.user, expected_user);
        }
    }

    /// import() should never panic regardless of input.
    #[test]
    fn import_never_panics(
        events in strategy_event_vec(10),
        coverage in strategy_coverage_manifest(),
    ) {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("prop_import");
        let engine = engine();
        let ingest = IngestOutput { events, coverage };
        let _ = engine.import(
            ingest,
            "proptester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            None,
            &BundleProfile::Internal,
        );
    }

    /// merge() with a single source should produce the same number of events.
    #[test]
    fn merge_single_source_preserves_count(
        events in strategy_event_vec(15),
        coverage in strategy_coverage_manifest(),
    ) {
        let engine = engine();
        let count = events.len();
        let ingest = IngestOutput { events, coverage };

        let merged = engine.merge(
            vec![ingest],
            shiplog_engine::ConflictResolution::PreferFirst,
        ).unwrap();

        // Single source => no duplicates => same count
        prop_assert_eq!(merged.events.len(), count);
    }
}
