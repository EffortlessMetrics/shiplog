//! End-to-end pipeline integration tests for the shiplog engine.
//!
//! These tests exercise the full pipeline from JSON ingest through
//! workstream clustering, rendering, redaction, and output verification.
//! They complement the unit-level `integration.rs` and `determinism.rs`
//! suites by wiring real adapters together end-to-end.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::{Engine, WorkstreamSource};
use shiplog_ids::RunId;
use shiplog_ingest_json::JsonIngestor;
use shiplog_output_layout::{DIR_PROFILES, FILE_PACKET_MD, PROFILE_MANAGER, PROFILE_PUBLIC};
use shiplog_ports::{IngestOutput, Ingestor, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::WorkstreamsFile;
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::WorkstreamManager;
use std::io::Write;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    shiplog_testkit::pr_event(repo, number, title)
}

fn review_event(repo: &str, pr_number: u64, pr_title: &str) -> EventEnvelope {
    EventEnvelope {
        id: shiplog_ids::EventId::from_parts(["github", "review", repo, &pr_number.to_string()]),
        kind: EventKind::Review,
        occurred_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: pr_number,
            pull_title: pr_title.to_string(),
            submitted_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            state: "APPROVED".into(),
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec!["review".into()],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{pr_number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/{repo}/pulls/{pr_number}/reviews/1"
            )),
            opaque_id: Some("1".into()),
        },
    }
}

fn make_coverage(user: &str, run_id: &str) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId(run_id.into()),
        generated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        user: user.into(),
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

fn make_ingest(events: Vec<EventEnvelope>) -> IngestOutput {
    IngestOutput {
        events,
        coverage: make_coverage("tester", "e2e_run"),
    }
}

fn make_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"e2e-test-key")));
    Engine::new(renderer, clusterer, redactor)
}

fn make_engine_with_key(key: &[u8]) -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(DeterministicRedactor::new(key)));
    Engine::new(renderer, clusterer, redactor)
}

// ---------------------------------------------------------------------------
// 1. Full JSON-ingest-to-render pipeline (fixture files)
// ---------------------------------------------------------------------------

#[test]
fn full_json_ingest_and_render_pipeline() {
    let fixture_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/fixture");

    let ingestor = JsonIngestor {
        events_path: fixture_dir.join("ledger.events.jsonl"),
        coverage_path: fixture_dir.join("coverage.manifest.json"),
    };
    let ingest = ingestor.ingest().unwrap();

    assert_eq!(ingest.events.len(), 3, "fixture has 3 events");
    assert_eq!(ingest.coverage.user, "octo");

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("full_e2e");

    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "octo",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    assert!(outputs.ledger_events_jsonl.exists());
    assert!(outputs.coverage_manifest_json.exists());
    assert!(outputs.bundle_manifest_json.exists());

    let packet = std::fs::read_to_string(&outputs.packet_md).unwrap();
    assert!(!packet.is_empty(), "packet should not be empty");

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    assert_eq!(ledger.lines().count(), 3, "ledger should have 3 lines");

    let cov_json = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();
    let cov: CoverageManifest = serde_json::from_str(&cov_json).unwrap();
    assert_eq!(cov.user, "octo");
}

// ---------------------------------------------------------------------------
// 2. Round-trip: write events -> ingest from JSONL -> render -> verify
// ---------------------------------------------------------------------------

#[test]
fn round_trip_events_and_coverage_integrity() {
    let tmp = tempfile::tempdir().unwrap();

    let events = vec![
        pr_event("acme/payments", 42, "Payments ledger rewrite"),
        pr_event("acme/platform", 13, "Schema hardening"),
        review_event("acme/platform", 77, "CI stabilization"),
    ];
    let events_path = tmp.path().join("events.jsonl");
    {
        let mut f = std::fs::File::create(&events_path).unwrap();
        for ev in &events {
            writeln!(f, "{}", serde_json::to_string(ev).unwrap()).unwrap();
        }
    }

    let cov = make_coverage("roundtrip-user", "roundtrip_run");
    let cov_path = tmp.path().join("coverage.json");
    std::fs::write(&cov_path, serde_json::to_string_pretty(&cov).unwrap()).unwrap();

    let ingestor = JsonIngestor {
        events_path,
        coverage_path: cov_path,
    };
    let ingest = ingestor.ingest().unwrap();
    assert_eq!(ingest.events.len(), 3);

    let out = tmp.path().join("output");
    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "roundtrip-user",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    let parsed: Vec<EventEnvelope> = ledger
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(parsed.len(), 3);

    let repos: Vec<&str> = parsed.iter().map(|e| e.repo.full_name.as_str()).collect();
    assert!(repos.contains(&"acme/payments"));
    assert!(repos.contains(&"acme/platform"));

    let cov_json = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();
    let restored_cov: CoverageManifest = serde_json::from_str(&cov_json).unwrap();
    assert_eq!(restored_cov.user, "roundtrip-user");
    assert_eq!(restored_cov.mode, "merged");
}

// ---------------------------------------------------------------------------
// 3. Multi-workstream clustering and rendering
// ---------------------------------------------------------------------------

#[test]
fn multi_workstream_clustering_and_rendering() {
    let events = vec![
        pr_event("acme/frontend", 1, "Add login page"),
        pr_event("acme/frontend", 2, "Fix CSS layout"),
        pr_event("acme/backend", 3, "Add REST API"),
        pr_event("acme/backend", 4, "Fix auth middleware"),
        pr_event("acme/infra", 5, "Setup CI pipeline"),
    ];
    let ingest = make_ingest(events);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("multi_ws");

    let engine = make_engine();
    let (outputs, ws_source) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Generated));

    let suggested_path = WorkstreamManager::suggested_path(&out);
    assert!(
        suggested_path.exists(),
        "suggested workstreams should exist"
    );

    let ws_text = std::fs::read_to_string(&suggested_path).unwrap();
    let ws: WorkstreamsFile = serde_yaml::from_str(&ws_text).unwrap();
    assert_eq!(
        ws.workstreams.len(),
        3,
        "should have 3 workstreams (frontend, backend, infra)"
    );

    let packet = std::fs::read_to_string(&outputs.packet_md).unwrap();
    assert!(packet.contains("acme/frontend"));
    assert!(packet.contains("acme/backend"));
    assert!(packet.contains("acme/infra"));
}

// ---------------------------------------------------------------------------
// 4. Redaction pipeline
// ---------------------------------------------------------------------------

#[test]
fn redaction_pipeline_strips_fields_for_profiles() {
    let events = vec![
        pr_event("acme/secret-repo", 1, "Confidential feature"),
        pr_event("acme/public-repo", 2, "Open source fix"),
    ];
    let ingest = make_ingest(events);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("redaction");

    let engine = make_engine();
    engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let internal = std::fs::read_to_string(out.join("packet.md")).unwrap();
    assert!(
        internal.contains("Confidential feature"),
        "internal packet should contain original titles"
    );

    let manager_path = out
        .join(DIR_PROFILES)
        .join(PROFILE_MANAGER)
        .join(FILE_PACKET_MD);
    let manager = std::fs::read_to_string(&manager_path).unwrap();
    assert!(!manager.is_empty(), "manager profile should not be empty");

    let public_path = out
        .join(DIR_PROFILES)
        .join(PROFILE_PUBLIC)
        .join(FILE_PACKET_MD);
    let public = std::fs::read_to_string(&public_path).unwrap();
    assert!(!public.is_empty(), "public profile should not be empty");
    assert_ne!(
        internal, public,
        "public profile should differ from internal"
    );
}

#[test]
fn different_redaction_keys_produce_different_outputs() {
    let events = vec![pr_event("acme/repo", 1, "Secret feature work")];

    let dir1 = tempfile::tempdir().unwrap();
    let out1 = dir1.path().join("key1");
    let dir2 = tempfile::tempdir().unwrap();
    let out2 = dir2.path().join("key2");

    let engine1 = make_engine_with_key(b"key-alpha");
    let engine2 = make_engine_with_key(b"key-bravo");

    let ingest1 = make_ingest(events.clone());
    let ingest2 = make_ingest(events);

    engine1
        .run(
            ingest1,
            "tester",
            "2025-01-01..2025-04-01",
            &out1,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();
    engine2
        .run(
            ingest2,
            "tester",
            "2025-01-01..2025-04-01",
            &out2,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let pub1 = std::fs::read_to_string(
        out1.join(DIR_PROFILES)
            .join(PROFILE_PUBLIC)
            .join(FILE_PACKET_MD),
    )
    .unwrap();
    let pub2 = std::fs::read_to_string(
        out2.join(DIR_PROFILES)
            .join(PROFILE_PUBLIC)
            .join(FILE_PACKET_MD),
    )
    .unwrap();

    assert_ne!(
        pub1, pub2,
        "different redaction keys should produce different public profiles"
    );
}

// ---------------------------------------------------------------------------
// 5. Empty / edge-case inputs
// ---------------------------------------------------------------------------

#[test]
fn empty_events_produces_valid_output() {
    let ingest = make_ingest(vec![]);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("empty");

    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "nobody",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    assert!(outputs.ledger_events_jsonl.exists());
    assert!(outputs.coverage_manifest_json.exists());

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    assert_eq!(
        ledger.lines().count(),
        0,
        "empty ledger should have 0 lines"
    );
}

#[test]
fn empty_coverage_slices_produces_valid_output() {
    let events = vec![pr_event("acme/foo", 1, "feature")];
    let mut coverage = make_coverage("tester", "empty_slices_run");
    coverage.slices = vec![];
    let ingest = IngestOutput { events, coverage };

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("empty_slices");

    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    let cov_json = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();
    let cov: CoverageManifest = serde_json::from_str(&cov_json).unwrap();
    assert!(cov.slices.is_empty());
}

#[test]
fn import_with_empty_events_and_no_workstreams() {
    let ingest = make_ingest(vec![]);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_empty");

    let engine = make_engine();
    let (outputs, ws_source) = engine
        .import(
            ingest,
            "importer",
            "2025-01-01..2025-02-01",
            &out,
            false,
            None,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Generated));
    assert!(outputs.packet_md.exists());
    assert!(outputs.ledger_events_jsonl.exists());
}

// ---------------------------------------------------------------------------
// 6. Stress tests
// ---------------------------------------------------------------------------

#[test]
fn stress_test_many_events_succeeds() {
    let events: Vec<EventEnvelope> = (0..200)
        .map(|i| {
            pr_event(
                &format!("org/repo-{}", i % 10),
                i as u64,
                &format!("PR #{i}"),
            )
        })
        .collect();
    let ingest = make_ingest(events);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("stress");

    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "stress-tester",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    assert_eq!(ledger.lines().count(), 200);

    let ws_path = WorkstreamManager::suggested_path(&out);
    let ws_text = std::fs::read_to_string(&ws_path).unwrap();
    let ws: WorkstreamsFile = serde_yaml::from_str(&ws_text).unwrap();
    assert_eq!(ws.workstreams.len(), 10);
}

#[test]
fn stress_test_mixed_event_types() {
    let mut events: Vec<EventEnvelope> = Vec::new();
    for i in 0..50u64 {
        events.push(pr_event("acme/app", i, &format!("PR #{i}")));
    }
    for i in 0..50u64 {
        events.push(review_event(
            "acme/app",
            100 + i,
            &format!("Review PR #{i}"),
        ));
    }
    let ingest = make_ingest(events);

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("mixed_stress");

    let engine = make_engine();
    let (outputs, _) = engine
        .run(
            ingest,
            "mixed-tester",
            "2025-01-01..2025-04-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    assert_eq!(ledger.lines().count(), 100);

    let packet = std::fs::read_to_string(&outputs.packet_md).unwrap();
    assert!(!packet.is_empty());
}

// ---------------------------------------------------------------------------
// 7. Deterministic output across runs
// ---------------------------------------------------------------------------

#[test]
fn deterministic_output_across_runs() {
    let events = vec![
        pr_event("acme/frontend", 1, "Login page"),
        pr_event("acme/backend", 2, "REST API"),
        review_event("acme/backend", 3, "Code review"),
    ];

    let mut packets = Vec::new();
    let mut ledgers = Vec::new();
    for _ in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("det");

        let ingest = make_ingest(events.clone());
        let engine = make_engine();
        let (outputs, _) = engine
            .run(
                ingest,
                "det-user",
                "2025-01-01..2025-04-01",
                &out,
                false,
                &BundleProfile::Internal,
            )
            .unwrap();

        packets.push(std::fs::read_to_string(&outputs.packet_md).unwrap());
        ledgers.push(std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap());
    }

    for i in 1..3 {
        assert_eq!(
            packets[0], packets[i],
            "packet.md must be identical on run {i}"
        );
        assert_eq!(
            ledgers[0], ledgers[i],
            "ledger must be identical on run {i}"
        );
    }
}

#[test]
fn deterministic_import_pipeline() {
    let events = vec![
        pr_event("acme/app", 1, "Feature A"),
        pr_event("acme/lib", 2, "Feature B"),
    ];

    let mut packets = Vec::new();
    for _ in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("det_import");

        let ingest = make_ingest(events.clone());
        let engine = make_engine();
        let (outputs, _) = engine
            .import(
                ingest,
                "det-importer",
                "2025-01-01..2025-04-01",
                &out,
                false,
                None,
                &BundleProfile::Internal,
            )
            .unwrap();

        packets.push(std::fs::read_to_string(&outputs.packet_md).unwrap());
    }

    for i in 1..3 {
        assert_eq!(
            packets[0], packets[i],
            "import packet.md must be identical on run {i}"
        );
    }
}
