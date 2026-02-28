//! Error-path tests for the shiplog-engine orchestration pipeline.
//!
//! Verifies that errors from renderers, clusterers, redactors, and filesystem
//! operations propagate correctly through `run()`, `import()`, and `refresh()`.

use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::Engine;
use shiplog_ids::RunId;
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::WorkstreamManager;

// ---------------------------------------------------------------------------
// Stubs
// ---------------------------------------------------------------------------

struct FailingRenderer;
impl Renderer for FailingRenderer {
    fn render_packet_markdown(
        &self,
        _user: &str,
        _window_label: &str,
        _events: &[EventEnvelope],
        _workstreams: &WorkstreamsFile,
        _coverage: &CoverageManifest,
    ) -> Result<String> {
        anyhow::bail!("renderer exploded")
    }
}

struct FailingClusterer;
impl WorkstreamClusterer for FailingClusterer {
    fn cluster(&self, _events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
        anyhow::bail!("clusterer exploded")
    }
}

struct FailingRedactor;
impl Redactor for FailingRedactor {
    fn redact_events(
        &self,
        _events: &[EventEnvelope],
        _profile: &str,
    ) -> Result<Vec<EventEnvelope>> {
        anyhow::bail!("redactor exploded")
    }
    fn redact_workstreams(&self, _ws: &WorkstreamsFile, _profile: &str) -> Result<WorkstreamsFile> {
        anyhow::bail!("redactor exploded")
    }
}

struct NoopRedactor;
impl Redactor for NoopRedactor {
    fn redact_events(
        &self,
        events: &[EventEnvelope],
        _profile: &str,
    ) -> Result<Vec<EventEnvelope>> {
        Ok(events.to_vec())
    }
    fn redact_workstreams(&self, ws: &WorkstreamsFile, _profile: &str) -> Result<WorkstreamsFile> {
        Ok(ws.clone())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("error_test".into()),
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        user: "tester".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
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
        coverage: make_coverage(),
    }
}

fn one_event() -> Vec<EventEnvelope> {
    vec![shiplog_testkit::pr_event("acme/foo", 1, "test PR")]
}

fn real_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(NoopRedactor));
    Engine::new(renderer, clusterer, redactor)
}

fn empty_workstreams() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    }
}

// ---------------------------------------------------------------------------
// import() error propagation
// ---------------------------------------------------------------------------

#[test]
fn import_propagates_renderer_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_fail_render");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(FailingRenderer));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(NoopRedactor));
    let engine = Engine::new(renderer, clusterer, redactor);

    let result = engine.import(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        None,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected renderer error"),
    };
    assert!(
        err.to_string().contains("renderer exploded"),
        "unexpected: {err}"
    );
}

#[test]
fn import_propagates_redactor_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_fail_redact");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(FailingRedactor));
    let engine = Engine::new(renderer, clusterer, redactor);

    let result = engine.import(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        None,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected redactor error"),
    };
    assert!(
        err.to_string().contains("redactor exploded"),
        "unexpected: {err}"
    );
}

#[test]
fn import_propagates_clusterer_error_when_no_workstreams() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_fail_cluster");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(FailingClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(NoopRedactor));
    let engine = Engine::new(renderer, clusterer, redactor);

    // No pre-existing workstreams and None passed → must cluster → error
    let result = engine.import(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        None,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected clusterer error"),
    };
    assert!(
        err.to_string().contains("clusterer exploded"),
        "unexpected: {err}"
    );
}

// ---------------------------------------------------------------------------
// refresh() error propagation
// ---------------------------------------------------------------------------

#[test]
fn refresh_propagates_renderer_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_fail_render");
    std::fs::create_dir_all(&out).unwrap();

    // Pre-write workstreams so refresh passes the "no workstreams" check
    shiplog_workstream_layout::write_workstreams(
        &WorkstreamManager::curated_path(&out),
        &empty_workstreams(),
    )
    .unwrap();

    let renderer: &'static dyn Renderer = Box::leak(Box::new(FailingRenderer));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(NoopRedactor));
    let engine = Engine::new(renderer, clusterer, redactor);

    let result = engine.refresh(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected renderer error"),
    };
    assert!(
        err.to_string().contains("renderer exploded"),
        "unexpected: {err}"
    );
}

#[test]
fn refresh_propagates_redactor_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_fail_redact");
    std::fs::create_dir_all(&out).unwrap();

    shiplog_workstream_layout::write_workstreams(
        &WorkstreamManager::curated_path(&out),
        &empty_workstreams(),
    )
    .unwrap();

    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(FailingRedactor));
    let engine = Engine::new(renderer, clusterer, redactor);

    let result = engine.refresh(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected redactor error"),
    };
    assert!(
        err.to_string().contains("redactor exploded"),
        "unexpected: {err}"
    );
}

#[test]
fn refresh_errors_on_corrupted_workstreams_yaml() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_corrupted_yaml");
    std::fs::create_dir_all(&out).unwrap();

    // Write invalid YAML to the curated path
    std::fs::write(WorkstreamManager::curated_path(&out), "{{{{not yaml").unwrap();

    let engine = real_engine();
    let result = engine.refresh(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    assert!(result.is_err(), "corrupted YAML should cause an error");
    let err = match result {
        Err(e) => e,
        Ok(_) => unreachable!(),
    };
    assert!(
        err.to_string().contains("parse curated workstreams yaml"),
        "unexpected: {err}"
    );
}

#[test]
fn refresh_errors_on_corrupted_suggested_yaml_when_no_curated() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_corrupted_suggested");
    std::fs::create_dir_all(&out).unwrap();

    // Write invalid YAML to the suggested path (no curated exists)
    std::fs::write(
        WorkstreamManager::suggested_path(&out),
        "not: [valid: yaml: {",
    )
    .unwrap();

    let engine = real_engine();
    let result = engine.refresh(
        make_ingest(one_event()),
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    assert!(result.is_err(), "corrupted suggested YAML should error");
    let err = match result {
        Err(e) => e,
        Ok(_) => unreachable!(),
    };
    assert!(
        err.to_string()
            .contains("parse suggested workstreams yaml"),
        "unexpected: {err}"
    );
}

// ---------------------------------------------------------------------------
// Empty event list edge cases across all commands
// ---------------------------------------------------------------------------

#[test]
fn run_empty_events_produces_zero_line_ledger() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("empty_ledger");

    let engine = real_engine();
    let (outputs, _) = engine
        .run(
            make_ingest(vec![]),
            "nobody",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    assert!(
        ledger.trim().is_empty(),
        "empty events should produce empty ledger"
    );
}

#[test]
fn import_empty_events_with_provided_workstreams_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_empty_ws");

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "orphan"]),
            title: "Orphan Workstream".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };

    let engine = real_engine();
    let (outputs, _) = engine
        .import(
            make_ingest(vec![]),
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            Some(ws),
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    let content = std::fs::read_to_string(&outputs.packet_md).unwrap();
    assert!(
        content.contains("Orphan Workstream"),
        "workstream title should appear in packet"
    );
}

// ---------------------------------------------------------------------------
// merge() error paths
// ---------------------------------------------------------------------------

#[test]
fn merge_single_input_succeeds() {
    let engine = real_engine();
    let result = engine.merge(
        vec![make_ingest(one_event())],
        shiplog_engine::ConflictResolution::PreferFirst,
    );
    assert!(result.is_ok());
    assert_eq!(result.unwrap().events.len(), 1);
}
