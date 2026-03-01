//! Cross-platform path handling tests for shiplog-engine.
//!
//! Ensures the engine pipeline works correctly with paths containing spaces,
//! unicode characters, and special characters on all platforms.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::Engine;
use shiplog_ids::RunId;
use shiplog_output_layout::{DIR_PROFILES, FILE_PACKET_MD, PROFILE_MANAGER, PROFILE_PUBLIC};
use shiplog_ports::{IngestOutput, Redactor, Renderer, WorkstreamClusterer};
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_workstream_cluster::RepoClusterer;

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
        coverage: make_coverage("tester", "cross_plat_run"),
    }
}

fn real_engine() -> Engine<'static> {
    let renderer: &'static dyn Renderer = Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"test-key")));
    Engine::new(renderer, clusterer, redactor)
}

// ---------------------------------------------------------------------------
// Paths with spaces
// ---------------------------------------------------------------------------

#[test]
fn run_succeeds_with_spaces_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("path with spaces").join("output dir");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat: spaces")]);

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

    assert!(
        outputs.packet_md.exists(),
        "packet.md missing with spaces in path"
    );
    assert!(outputs.ledger_events_jsonl.exists());
    assert!(outputs.coverage_manifest_json.exists());
    assert!(outputs.bundle_manifest_json.exists());

    // Profile directories should also be created correctly
    let manager_md = out
        .join(DIR_PROFILES)
        .join(PROFILE_MANAGER)
        .join(FILE_PACKET_MD);
    let public_md = out
        .join(DIR_PROFILES)
        .join(PROFILE_PUBLIC)
        .join(FILE_PACKET_MD);
    assert!(
        manager_md.exists(),
        "manager profile missing with spaces in path"
    );
    assert!(
        public_md.exists(),
        "public profile missing with spaces in path"
    );
}

// ---------------------------------------------------------------------------
// Paths with unicode characters
// ---------------------------------------------------------------------------

#[test]
fn run_succeeds_with_unicode_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("日本語パス").join("données");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat: unicode")]);

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

    assert!(
        outputs.packet_md.exists(),
        "packet.md missing with unicode in path"
    );
    assert!(outputs.ledger_events_jsonl.exists());
    assert!(outputs.coverage_manifest_json.exists());
}

// ---------------------------------------------------------------------------
// Paths with special characters (parentheses, hyphens, dots)
// ---------------------------------------------------------------------------

#[test]
fn run_succeeds_with_special_chars_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("project (v2.0)").join("out-put_dir");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat: special chars")]);

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

    assert!(outputs.packet_md.exists());
    assert!(outputs.bundle_manifest_json.exists());
}

// ---------------------------------------------------------------------------
// Deeply nested paths
// ---------------------------------------------------------------------------

#[test]
fn run_succeeds_with_deeply_nested_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir
        .path()
        .join("level1")
        .join("level2")
        .join("level3")
        .join("level4")
        .join("output");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "feat: deep nesting")]);

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

    assert!(outputs.packet_md.exists());
    // Verify output path is under the expected directory
    assert!(outputs.out_dir.starts_with(dir.path()));
}

// ---------------------------------------------------------------------------
// Zip creation with special path characters
// ---------------------------------------------------------------------------

#[test]
fn zip_creation_with_spaces_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("zip test dir");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "zip spaces")]);

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
    let zip = outputs.zip_path.as_ref().unwrap();
    assert!(zip.exists(), "zip file missing with spaces in path");
}

#[test]
fn zip_creation_with_unicode_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("archive_出力");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "zip unicode")]);

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

// ---------------------------------------------------------------------------
// Output paths use Path::join (not string concatenation)
// ---------------------------------------------------------------------------

#[test]
fn output_paths_are_children_of_out_dir() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("verify joins");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "verify joins")]);

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

    // All output paths must be descendants of out_dir
    assert!(outputs.packet_md.starts_with(&out));
    assert!(outputs.ledger_events_jsonl.starts_with(&out));
    assert!(outputs.coverage_manifest_json.starts_with(&out));
    assert!(outputs.bundle_manifest_json.starts_with(&out));
    assert!(outputs.workstreams_yaml.starts_with(&out));
}

// ---------------------------------------------------------------------------
// Refresh with special paths
// ---------------------------------------------------------------------------

#[test]
fn refresh_works_with_spaces_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("refresh path with spaces");
    std::fs::create_dir_all(&out).unwrap();

    // Pre-write curated workstreams
    let ws = shiplog_schema::workstream::WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };
    shiplog_workstream_layout::write_workstreams(
        &shiplog_workstream_layout::WorkstreamManager::curated_path(&out),
        &ws,
    )
    .unwrap();

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "refresh spaces")]);

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

// ---------------------------------------------------------------------------
// Import with special paths
// ---------------------------------------------------------------------------

#[test]
fn import_works_with_unicode_in_path() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("importé_données");

    let engine = real_engine();
    let ingest = make_ingest(vec![pr_event("acme/foo", 1, "import unicode")]);

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
