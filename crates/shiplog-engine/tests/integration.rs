//! Integration tests for the shiplog-engine orchestration pipeline.

use anyhow::Result;
use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::{Engine, WorkstreamSource};
use shiplog_ids::RunId;
use shiplog_output_layout::{DIR_PROFILES, FILE_PACKET_MD, PROFILE_MANAGER, PROFILE_PUBLIC};
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::WorkstreamManager;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    shiplog_testkit::pr_event(repo, number, title)
}

fn make_coverage(user: &str, run_id: &str) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId(run_id.into()),
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        user: user.into(),
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
        coverage: make_coverage("tester", "integration_run"),
    }
}

fn real_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"test-key")));
    Engine::new(renderer, clusterer, redactor)
}

// Stub implementations for testing error paths and custom behavior.

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

struct CountingRenderer {
    counter: std::sync::atomic::AtomicUsize,
}
impl CountingRenderer {
    fn new() -> Self {
        Self {
            counter: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    fn count(&self) -> usize {
        self.counter.load(std::sync::atomic::Ordering::Relaxed)
    }
}
impl Renderer for CountingRenderer {
    fn render_packet_markdown(
        &self,
        _user: &str,
        _window_label: &str,
        _events: &[EventEnvelope],
        _workstreams: &WorkstreamsFile,
        _coverage: &CoverageManifest,
    ) -> Result<String> {
        self.counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok("# mock packet".into())
    }
}

// ---------------------------------------------------------------------------
// run() tests
// ---------------------------------------------------------------------------

#[test]
fn run_with_empty_events_produces_valid_output() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("empty_run");

    let engine = real_engine();
    let ingest = make_ingest(vec![]);

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
    assert!(outputs.bundle_manifest_json.exists());
    assert!(outputs.zip_path.is_none());
}

#[test]
fn run_generates_workstream_suggested_when_none_exists() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("gen_ws");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat: add login")]);

    let (_, ws_source) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Generated));
    assert!(WorkstreamManager::suggested_path(&out).exists());
}

#[test]
fn run_uses_curated_workstreams_when_present() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("curated_ws");
    std::fs::create_dir_all(&out).unwrap();

    // Pre-write curated workstreams
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "curated"]),
            title: "Curated Feature".into(),
            summary: Some("User-edited workstream".into()),
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: 1,
                reviews: 0,
                manual_events: 0,
            },
            events: vec![],
            receipts: vec![],
        }],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::curated_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "some pr")]);

    let (_, ws_source) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Curated));
}

#[test]
fn run_uses_suggested_workstreams_when_only_suggested_exists() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("suggested_ws");
    std::fs::create_dir_all(&out).unwrap();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "suggested"]),
            title: "Suggested Feature".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::suggested_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "some pr")]);

    let (_, ws_source) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Suggested));
}

#[test]
fn run_creates_manager_and_public_profile_packets() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("profiles");

    let engine = real_engine();
    let ingest = make_ingest(vec![
        pr_event("acme/foo", 1, "Add feature"),
        pr_event("acme/bar", 2, "Fix bug"),
    ]);

    engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let manager_md = out
        .join(DIR_PROFILES)
        .join(PROFILE_MANAGER)
        .join(FILE_PACKET_MD);
    let public_md = out
        .join(DIR_PROFILES)
        .join(PROFILE_PUBLIC)
        .join(FILE_PACKET_MD);
    assert!(manager_md.exists(), "manager profile packet missing");
    assert!(public_md.exists(), "public profile packet missing");

    // Profile packets should not be empty
    let manager_content = std::fs::read_to_string(&manager_md).unwrap();
    let public_content = std::fs::read_to_string(&public_md).unwrap();
    assert!(!manager_content.is_empty());
    assert!(!public_content.is_empty());
}

#[test]
fn run_with_zip_true_creates_archive_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("zip_test");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feature")]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            true,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.zip_path.is_some());
    assert!(outputs.zip_path.as_ref().unwrap().exists());
}

#[test]
fn run_with_zip_false_produces_no_archive() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("nozip");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feature")]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.zip_path.is_none());
}

#[test]
fn run_with_multiple_repos_clusters_separately() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("multi_repo");

    let engine = real_engine();
    let ingest = make_ingest(vec![
        pr_event("acme/frontend", 1, "Add button"),
        pr_event("acme/frontend", 2, "Fix CSS"),
        pr_event("acme/backend", 3, "Add API"),
        pr_event("acme/backend", 4, "Fix auth"),
    ]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    // Packet should reference both repos
    let content = std::fs::read_to_string(&outputs.packet_md).unwrap();
    assert!(
        content.contains("acme/frontend"),
        "missing frontend repo in packet"
    );
    assert!(
        content.contains("acme/backend"),
        "missing backend repo in packet"
    );
}

#[test]
fn run_ledger_contains_all_events() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("ledger");

    let engine = real_engine();
    let events = vec![
        pr_event("acme/foo", 1, "first"),
        pr_event("acme/foo", 2, "second"),
        pr_event("acme/foo", 3, "third"),
    ];
    let ingest = make_ingest(events);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let ledger = std::fs::read_to_string(&outputs.ledger_events_jsonl).unwrap();
    let lines: Vec<&str> = ledger.lines().collect();
    assert_eq!(lines.len(), 3, "ledger should have 3 events");
}

// ---------------------------------------------------------------------------
// Renderer is called 3 times per run (internal + manager + public)
// ---------------------------------------------------------------------------

#[test]
fn run_calls_renderer_three_times() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("counting");

    let renderer = Box::leak(Box::new(CountingRenderer::new()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(NoopRedactor));

    let engine = Engine::new(renderer as &dyn Renderer, clusterer, redactor);
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "test")]);

    engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    // Once for internal packet, once for manager profile, once for public profile
    assert_eq!(
        renderer.count(),
        3,
        "renderer should be called 3 times (internal + manager + public)"
    );
}

// ---------------------------------------------------------------------------
// Error propagation tests
// ---------------------------------------------------------------------------

#[test]
fn run_propagates_renderer_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fail_render");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(FailingRenderer));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(DeterministicRedactor::new(b"key")));

    let engine = Engine::new(renderer, clusterer, redactor);
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "test")]);

    let result = engine.run(
        ingest,
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
        format!("{err:#}").contains("renderer exploded"),
        "unexpected error: {err}"
    );
}

#[test]
fn run_propagates_clusterer_error_when_no_workstreams_file() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fail_cluster");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(FailingClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(DeterministicRedactor::new(b"key")));

    let engine = Engine::new(renderer, clusterer, redactor);
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "test")]);

    let result = engine.run(
        ingest,
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected clusterer error"),
    };

    assert!(
        format!("{err:#}").contains("clusterer exploded"),
        "unexpected error: {err}"
    );
}

#[test]
fn run_propagates_redactor_error() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("fail_redact");

    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor = Box::leak(Box::new(FailingRedactor));

    let engine = Engine::new(renderer, clusterer, redactor);
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "test")]);

    let result = engine.run(
        ingest,
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
        format!("{err:#}").contains("redactor exploded"),
        "unexpected error: {err}"
    );
}

// ---------------------------------------------------------------------------
// import() tests
// ---------------------------------------------------------------------------

#[test]
fn import_with_provided_workstreams_writes_curated() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_curated");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "imported pr")]);

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "imported"]),
            title: "Imported Work".into(),
            summary: Some("From external source".into()),
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: 1,
                reviews: 0,
                manual_events: 0,
            },
            events: vec![],
            receipts: vec![],
        }],
    };

    let (outputs, ws_source) = engine
        .import(
            ingest,
            "importer",
            "2025-01-01..2025-02-01",
            &out,
            false,
            Some(ws),
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(matches!(ws_source, WorkstreamSource::Curated));
    assert!(WorkstreamManager::curated_path(&out).exists());
    assert!(outputs.packet_md.exists());
}

#[test]
fn import_without_workstreams_generates_new() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_gen");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "import pr")]);

    let (_, ws_source) = engine
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
}

#[test]
fn import_with_empty_events_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_empty");

    let engine = real_engine();
    let ingest = make_ingest(vec![]);

    let (outputs, _) = engine
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

    assert!(outputs.packet_md.exists());
    assert!(outputs.ledger_events_jsonl.exists());
}

#[test]
fn import_with_zip_creates_archive() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("import_zip");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "zip import")]);

    let (outputs, _) = engine
        .import(
            ingest,
            "importer",
            "2025-01-01..2025-02-01",
            &out,
            true,
            None,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.zip_path.is_some());
    assert!(outputs.zip_path.as_ref().unwrap().exists());
}

// ---------------------------------------------------------------------------
// refresh() tests
// ---------------------------------------------------------------------------

#[test]
fn refresh_with_curated_workstreams_preserves_curation() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_curated");
    std::fs::create_dir_all(&out).unwrap();

    // Pre-write curated workstreams
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "curated-refresh"]),
            title: "Curated Refresh".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::curated_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "refreshed")]);

    let outputs = engine
        .refresh(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    // Curated workstreams file should still be the curated one
    assert_eq!(
        outputs.workstreams_yaml,
        WorkstreamManager::curated_path(&out)
    );
}

#[test]
fn refresh_with_suggested_workstreams_uses_suggested() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_suggested");
    std::fs::create_dir_all(&out).unwrap();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: shiplog_ids::WorkstreamId::from_parts(["ws", "suggested-refresh"]),
            title: "Suggested Refresh".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::suggested_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "refreshed")]);

    let outputs = engine
        .refresh(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    assert_eq!(
        outputs.workstreams_yaml,
        WorkstreamManager::suggested_path(&out)
    );
}

#[test]
fn refresh_fails_when_no_workstreams_exist() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_none");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "orphan")]);

    let result = engine.refresh(
        ingest,
        "tester",
        "2025-01-01..2025-02-01",
        &out,
        false,
        &BundleProfile::Internal,
    );

    let err = match result {
        Err(e) => e,
        Ok(_) => panic!("expected refresh error when no workstreams exist"),
    };

    assert!(
        err.to_string().contains("No workstreams found"),
        "unexpected error: {err}"
    );
}

#[test]
fn refresh_with_empty_events_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_empty");
    std::fs::create_dir_all(&out).unwrap();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::curated_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![]);

    let outputs = engine
        .refresh(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
}

#[test]
fn refresh_with_zip_creates_archive() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh_zip");
    std::fs::create_dir_all(&out).unwrap();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    shiplog_workstream_layout::write_workstreams(&WorkstreamManager::curated_path(&out), &ws)
        .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "zip refresh")]);

    let outputs = engine
        .refresh(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            true,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.zip_path.is_some());
    assert!(outputs.zip_path.as_ref().unwrap().exists());
}

// ---------------------------------------------------------------------------
// merge() tests
// ---------------------------------------------------------------------------

#[test]
fn merge_deduplicates_identical_events() {
    let engine = real_engine();

    let event = pr_event("acme/foo", 1, "same pr");
    let cov = make_coverage("tester", "merge1");
    let ingest1 = IngestOutput {
        events: vec![event.clone()],
        coverage: cov.clone(),
    };
    let ingest2 = IngestOutput {
        events: vec![event],
        coverage: make_coverage("tester", "merge2"),
    };

    let merged = engine
        .merge(
            vec![ingest1, ingest2],
            shiplog_engine::ConflictResolution::PreferFirst,
        )
        .unwrap();

    assert_eq!(merged.events.len(), 1, "duplicates should be removed");
}

#[test]
fn merge_combines_distinct_events() {
    let engine = real_engine();

    let ingest1 = IngestOutput {
        events: vec![pr_event("acme/foo", 1, "first")],
        coverage: make_coverage("tester", "m1"),
    };
    let ingest2 = IngestOutput {
        events: vec![pr_event("acme/bar", 2, "second")],
        coverage: make_coverage("tester", "m2"),
    };

    let merged = engine
        .merge(
            vec![ingest1, ingest2],
            shiplog_engine::ConflictResolution::PreferFirst,
        )
        .unwrap();

    assert_eq!(merged.events.len(), 2);
}

#[test]
fn merge_empty_input_is_error() {
    let engine = real_engine();

    let err = engine
        .merge(vec![], shiplog_engine::ConflictResolution::PreferFirst)
        .unwrap_err();

    assert!(err.to_string().contains("No ingest outputs to merge"));
}

// ---------------------------------------------------------------------------
// Bundle profile tests
// ---------------------------------------------------------------------------

#[test]
fn run_with_manager_profile() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("manager_profile");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feature")]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Manager,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    assert!(outputs.bundle_manifest_json.exists());
}

#[test]
fn run_with_public_profile() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("public_profile");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feature")]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Public,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
    assert!(outputs.bundle_manifest_json.exists());
}

// ---------------------------------------------------------------------------
// Idempotency: running the engine twice on the same dir should not error
// ---------------------------------------------------------------------------

#[test]
fn run_is_idempotent_on_same_dir() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("idempotent");

    let engine = real_engine();
    let ingest1 = make_ingest(vec![pr_event("acme/foo", 1, "first run")]);
    let ingest2 = make_ingest(vec![pr_event("acme/foo", 1, "first run")]);

    engine
        .run(
            ingest1,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();
    let (outputs, _) = engine
        .run(
            ingest2,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    assert!(outputs.packet_md.exists());
}

// ---------------------------------------------------------------------------
// Coverage manifest roundtrip
// ---------------------------------------------------------------------------

#[test]
fn coverage_manifest_is_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("coverage_json");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat")]);

    let (outputs, _) = engine
        .run(
            ingest,
            "tester",
            "2025-01-01..2025-02-01",
            &out,
            false,
            &BundleProfile::Internal,
        )
        .unwrap();

    let json_str = std::fs::read_to_string(&outputs.coverage_manifest_json).unwrap();
    let parsed: CoverageManifest = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed.user, "tester");
    assert_eq!(parsed.mode, "merged");
}
