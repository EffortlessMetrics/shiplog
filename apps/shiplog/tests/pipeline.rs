//! Cross-crate integration tests exercising the full engine pipeline
//! with real fixture data.

use anyhow::Result;
use chrono::{NaiveDate, Utc};
use shiplog_engine::{ConflictResolution, Engine, WorkstreamSource};
use shiplog_ingest_json::JsonIngestor;
use shiplog_ingest_manual::ManualIngestor;
use shiplog_ports::{IngestOutput, Ingestor, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::{
    ManualDate, ManualEventEntry, ManualEventType, ManualEventsFile,
};
use shiplog_workstream_cluster::RepoClusterer;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ── helpers ─────────────────────────────────────────────────────────────────

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("examples/fixture")
}

fn fixture_ingest() -> IngestOutput {
    let fixtures = fixture_dir();
    let ing = JsonIngestor {
        events_path: fixtures.join("ledger.events.jsonl"),
        coverage_path: fixtures.join("coverage.manifest.json"),
    };
    ing.ingest().expect("fixture ingest failed")
}

fn test_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer =
        Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer =
        Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"integration-test-key")));
    Engine::new(renderer, clusterer, redactor)
}

// ── 1. JSON ingest → engine → markdown output ──────────────────────────────

#[test]
fn json_ingest_to_markdown_output() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();

    let (outputs, _ws_source) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    assert!(outputs.packet_md.exists(), "packet.md should exist");

    let packet = std::fs::read_to_string(&outputs.packet_md)?;

    // Verify expected markdown sections
    assert!(packet.contains("# Summary"), "packet should contain Summary heading");
    assert!(
        packet.contains("2025-01-01..2025-04-01"),
        "packet should reference the window label"
    );
    // Verify it contains PR-related content
    assert!(
        packet.contains("Payments ledger rewrite")
            || packet.contains("Schema hardening"),
        "packet should contain PR titles from fixtures"
    );

    Ok(())
}

// ── 2. JSON ingest → engine → JSON output (roundtrip) ──────────────────────

#[test]
fn json_ingest_to_json_output_roundtrips() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();
    let original_event_count = ingest.events.len();

    let (outputs, _) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // Re-read the written JSONL and coverage manifest
    let re_ingest = JsonIngestor {
        events_path: outputs.ledger_events_jsonl.clone(),
        coverage_path: outputs.coverage_manifest_json.clone(),
    };
    let reloaded = re_ingest.ingest()?;

    assert_eq!(
        reloaded.events.len(),
        original_event_count,
        "roundtripped event count should match"
    );

    // Verify each event parses back with the same ID
    let original_ids: Vec<_> = {
        let fresh = fixture_ingest();
        fresh.events.iter().map(|e| e.id.clone()).collect()
    };
    let reloaded_ids: Vec<_> = reloaded.events.iter().map(|e| e.id.clone()).collect();
    assert_eq!(original_ids, reloaded_ids, "event IDs should roundtrip");

    // Coverage manifest should also roundtrip
    assert_eq!(
        reloaded.coverage.run_id,
        fixture_ingest().coverage.run_id,
        "coverage run_id should roundtrip"
    );

    Ok(())
}

// ── 3. Multi-source ingest (JSON + manual) → combined output ────────────────

#[test]
fn multi_source_json_plus_manual_combined_output() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("multi_source_run");

    // Prepare manual events file
    let manual_path = tmp.path().join("manual_events.yaml");
    let manual_file = ManualEventsFile {
        version: 1,
        generated_at: Utc::now(),
        events: vec![ManualEventEntry {
            id: "manual-incident-1".to_string(),
            event_type: ManualEventType::Incident,
            date: ManualDate::Single(NaiveDate::from_ymd_opt(2025, 2, 15).unwrap()),
            title: "Production incident: payment gateway timeout".to_string(),
            description: Some("Resolved p95 latency spike in payment processing".to_string()),
            workstream: Some("reliability".to_string()),
            tags: vec!["incident".to_string(), "payments".to_string()],
            receipts: vec![],
            impact: Some("Reduced error rate from 5% to 0.1%".to_string()),
        }],
    };
    shiplog_ingest_manual::write_manual_events(&manual_path, &manual_file)?;

    // Ingest from both sources
    let json_ingest = fixture_ingest();

    let manual_ing = ManualIngestor::new(
        &manual_path,
        "octo".to_string(),
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    );
    let manual_ingest = manual_ing.ingest()?;

    // Merge sources via engine
    let engine = test_engine();
    let merged = engine.merge(
        vec![json_ingest, manual_ingest],
        ConflictResolution::PreferFirst,
    )?;

    // The merged output should contain events from both sources
    // JSON fixture has 3 events, manual has 1
    assert!(
        merged.events.len() >= 4,
        "merged should have at least 4 events (3 JSON + 1 manual), got {}",
        merged.events.len()
    );

    // Verify manual event is present
    let has_manual = merged
        .events
        .iter()
        .any(|e| matches!(e.payload, shiplog_schema::event::EventPayload::Manual(_)));
    assert!(has_manual, "merged output should contain manual events");

    // Run the full pipeline with merged data
    let (outputs, _) = engine.run(
        merged,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    let packet = std::fs::read_to_string(&outputs.packet_md)?;

    // Packet should reference the manual event
    assert!(
        packet.contains("payment gateway timeout")
            || packet.contains("incident")
            || packet.contains("manual"),
        "packet should include manual event content"
    );

    Ok(())
}

// ── 4. Coverage manifest generation ────────────────────────────────────────

#[test]
fn coverage_manifest_has_receipts_for_all_events() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();
    let event_count = ingest.events.len();

    let (outputs, _) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // Read back the coverage manifest
    let cov_text = std::fs::read_to_string(&outputs.coverage_manifest_json)?;
    let coverage: CoverageManifest = serde_json::from_str(&cov_text)?;

    // Coverage should reference the correct user and window
    assert_eq!(coverage.user, "octo");
    assert_eq!(
        coverage.window.since,
        NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()
    );
    assert_eq!(
        coverage.window.until,
        NaiveDate::from_ymd_opt(2025, 4, 1).unwrap()
    );

    // The total fetched across slices should account for all events
    let total_fetched: u64 = coverage.slices.iter().map(|s| s.fetched).sum();
    assert_eq!(
        total_fetched, event_count as u64,
        "fetched count in coverage slices should match event count"
    );

    // Coverage should have at least one slice
    assert!(
        !coverage.slices.is_empty(),
        "coverage should have at least one slice"
    );

    // Sources should be recorded
    assert!(
        !coverage.sources.is_empty(),
        "coverage should list at least one source"
    );

    Ok(())
}

// ── 5. Workstream clustering ───────────────────────────────────────────────

#[test]
fn workstream_clustering_groups_events_by_repo() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();

    let (outputs, ws_source) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // First run should generate workstreams (no pre-existing file)
    assert!(
        matches!(ws_source, WorkstreamSource::Generated),
        "expected Generated workstream source on first run"
    );

    // Read workstreams file
    let ws_text = std::fs::read_to_string(&outputs.workstreams_yaml)?;
    let ws: shiplog_schema::workstream::WorkstreamsFile = serde_yaml::from_str(&ws_text)?;

    // RepoClusterer groups by repo — fixture has 2 repos (acme/payments, acme/platform)
    assert!(
        ws.workstreams.len() >= 2,
        "expected at least 2 workstreams (one per repo), got {}",
        ws.workstreams.len()
    );

    // Each workstream should have at least one event
    for ws_entry in &ws.workstreams {
        assert!(
            !ws_entry.events.is_empty(),
            "workstream '{}' should have at least one event",
            ws_entry.title
        );
    }

    // Verify all fixture events are assigned to a workstream
    let total_assigned: usize = ws.workstreams.iter().map(|w| w.events.len()).sum();
    assert_eq!(
        total_assigned, 3,
        "all 3 fixture events should be assigned to workstreams"
    );

    Ok(())
}

// ── 6. Redaction pipeline ──────────────────────────────────────────────────

#[test]
fn redacted_output_strips_sensitive_data() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();

    let (_, _) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // Read the public profile packet
    let public_packet_path = out_dir
        .join("profiles")
        .join("public")
        .join("packet.md");
    assert!(
        public_packet_path.exists(),
        "public profile packet should exist"
    );

    let public_packet = std::fs::read_to_string(&public_packet_path)?;

    // Public profile should NOT contain original PR titles
    assert!(
        !public_packet.contains("Payments ledger rewrite"),
        "public packet should not contain original PR title 'Payments ledger rewrite'"
    );
    assert!(
        !public_packet.contains("Schema hardening for audit exports"),
        "public packet should not contain original PR title 'Schema hardening for audit exports'"
    );

    // Public profile should NOT contain original repo URLs
    assert!(
        !public_packet.contains("https://github.com/acme/payments/pull/42"),
        "public packet should not contain original PR link"
    );
    assert!(
        !public_packet.contains("https://github.com/acme/platform/pull/13"),
        "public packet should not contain original PR link"
    );

    // Manager profile should also exist
    let manager_packet_path = out_dir
        .join("profiles")
        .join("manager")
        .join("packet.md");
    assert!(
        manager_packet_path.exists(),
        "manager profile packet should exist"
    );

    // Internal packet should still contain original data
    let internal_packet = std::fs::read_to_string(out_dir.join("packet.md"))?;
    assert!(
        internal_packet.contains("Payments ledger rewrite")
            || internal_packet.contains("Schema hardening"),
        "internal packet should contain original PR titles"
    );

    Ok(())
}

// ── Additional: bundle manifest ────────────────────────────────────────────

#[test]
fn bundle_manifest_is_generated() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();
    let ingest = fixture_ingest();

    let (outputs, _) = engine.run(
        ingest,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    assert!(
        outputs.bundle_manifest_json.exists(),
        "bundle.manifest.json should exist"
    );

    let bundle_text = std::fs::read_to_string(&outputs.bundle_manifest_json)?;
    let bundle: serde_json::Value = serde_json::from_str(&bundle_text)?;

    // Bundle manifest should have file checksums
    assert!(
        bundle.get("files").is_some() || bundle.get("checksums").is_some(),
        "bundle manifest should contain file checksums"
    );

    Ok(())
}

// ── Additional: refresh preserves workstreams ──────────────────────────────

#[test]
fn refresh_preserves_existing_workstreams() -> Result<()> {
    let tmp = TempDir::new()?;
    let out_dir = tmp.path().join("run_fixture");

    let engine = test_engine();

    // First run: generates workstreams
    let ingest1 = fixture_ingest();
    let (outputs1, _) = engine.run(
        ingest1,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // Read workstreams from first run
    let ws_text_1 = std::fs::read_to_string(&outputs1.workstreams_yaml)?;

    // Refresh with the same data
    let ingest2 = fixture_ingest();
    let outputs2 = engine.refresh(
        ingest2,
        "octo",
        "2025-01-01..2025-04-01",
        &out_dir,
        false,
        &BundleProfile::Internal,
    )?;

    // Workstreams should be preserved
    let ws_text_2 = std::fs::read_to_string(&outputs2.workstreams_yaml)?;
    assert_eq!(
        ws_text_1, ws_text_2,
        "refresh should preserve the same workstreams file"
    );

    Ok(())
}
