//! Determinism tests for shiplog-engine.
//!
//! Verifies that the full pipeline produces byte-for-byte identical outputs
//! when given the same inputs across multiple runs.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::Engine;
use shiplog_ids::RunId;
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::EventEnvelope;
use shiplog_workstream_cluster::RepoClusterer;

const RUNS: usize = 3;

fn test_events() -> Vec<EventEnvelope> {
    vec![
        shiplog_testkit::pr_event("acme/frontend", 1, "Add login page"),
        shiplog_testkit::pr_event("acme/frontend", 2, "Fix CSS layout"),
        shiplog_testkit::pr_event("acme/backend", 3, "Add REST API"),
        shiplog_testkit::pr_event("acme/backend", 4, "Fix auth middleware"),
        shiplog_testkit::pr_event("acme/infra", 5, "Setup CI pipeline"),
    ]
}

fn test_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("determinism_test_run".into()),
        generated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        user: "testuser".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn test_ingest() -> IngestOutput {
    IngestOutput {
        events: test_events(),
        coverage: test_coverage(),
    }
}

fn make_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"determinism-key")));
    Engine::new(renderer, clusterer, redactor)
}

/// Run the engine in a fresh temp dir and return file contents.
fn run_pipeline() -> (String, String, String) {
    let dir = tempfile::tempdir().unwrap();
    let out_dir = dir.path().join("run");

    let engine = make_engine();
    let ingest = test_ingest();

    let (outputs, _) = engine
        .run(
            ingest,
            "testuser",
            "2025-01-01..2025-04-01",
            &out_dir,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let packet_md = std::fs::read_to_string(&outputs.packet_md).unwrap();
    let ledger_jsonl = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    let coverage_json = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();

    (packet_md, ledger_jsonl, coverage_json)
}

// ---------------------------------------------------------------------------
// Same events → same packet.md (byte-for-byte) across runs
// ---------------------------------------------------------------------------

#[test]
fn packet_md_byte_identical_across_runs() {
    let reference = run_pipeline();
    for i in 1..RUNS {
        let current = run_pipeline();
        assert_eq!(
            reference.0, current.0,
            "packet.md must be byte-identical on run {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Same events → same JSONL output across runs
// ---------------------------------------------------------------------------

#[test]
fn ledger_jsonl_byte_identical_across_runs() {
    let reference = run_pipeline();
    for i in 1..RUNS {
        let current = run_pipeline();
        assert_eq!(
            reference.1, current.1,
            "ledger.events.jsonl must be byte-identical on run {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Same events → same coverage manifest across runs
// ---------------------------------------------------------------------------

#[test]
fn coverage_manifest_byte_identical_across_runs() {
    let reference = run_pipeline();
    for i in 1..RUNS {
        let current = run_pipeline();
        assert_eq!(
            reference.2, current.2,
            "coverage.manifest.json must be byte-identical on run {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Profile packets are also deterministic
// ---------------------------------------------------------------------------

#[test]
fn profile_packets_deterministic_across_runs() {
    let mut manager_packets = Vec::new();
    let mut public_packets = Vec::new();

    for _ in 0..RUNS {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("run");

        let engine = make_engine();
        let ingest = test_ingest();

        engine
            .run(
                ingest,
                "testuser",
                "2025-01-01..2025-04-01",
                &out_dir,
                false,
                &BundleProfile::Internal,
            )
            .unwrap();

        let manager_md =
            std::fs::read_to_string(out_dir.join("profiles").join("manager").join("packet.md"))
                .unwrap();
        let public_md =
            std::fs::read_to_string(out_dir.join("profiles").join("public").join("packet.md"))
                .unwrap();

        manager_packets.push(manager_md);
        public_packets.push(public_md);
    }

    for i in 1..RUNS {
        assert_eq!(
            manager_packets[0], manager_packets[i],
            "manager profile packet must be identical on run {i}"
        );
        assert_eq!(
            public_packets[0], public_packets[i],
            "public profile packet must be identical on run {i}"
        );
    }
}

// ---------------------------------------------------------------------------
// Import pipeline is also deterministic
// ---------------------------------------------------------------------------

#[test]
fn import_pipeline_deterministic_across_runs() {
    let mut packets = Vec::new();

    for _ in 0..RUNS {
        let dir = tempfile::tempdir().unwrap();
        let out_dir = dir.path().join("import");

        let engine = make_engine();
        let ingest = test_ingest();

        let (outputs, _) = engine
            .import(
                ingest,
                "testuser",
                "2025-01-01..2025-04-01",
                &out_dir,
                false,
                None,
                &BundleProfile::Internal,
            )
            .unwrap();

        let packet_md = std::fs::read_to_string(&outputs.packet_md).unwrap();
        packets.push(packet_md);
    }

    for i in 1..RUNS {
        assert_eq!(
            packets[0], packets[i],
            "import packet.md must be identical on run {i}"
        );
    }
}
