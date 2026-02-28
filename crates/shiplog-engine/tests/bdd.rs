//! BDD-style integration tests for the shiplog engine pipeline.
//!
//! Each scenario exercises the full pipeline: ingest → cluster → render.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::{Engine, WorkstreamSource};
use shiplog_ids::{EventId, RunId};
use shiplog_output_layout::{DIR_PROFILES, FILE_PACKET_MD, PROFILE_MANAGER, PROFILE_PUBLIC};
use shiplog_ports::IngestOutput;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_testkit::bdd::assertions::*;
use shiplog_testkit::bdd::Scenario;
use shiplog_workstream_cluster::RepoClusterer;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.to_string(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
            additions: Some(10),
            deletions: Some(2),
            changed_files: Some(3),
            touched_paths_hint: vec![],
            window: Some(TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            }),
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/...".into()),
            opaque_id: None,
        },
    }
}

fn test_ingest(events: Vec<EventEnvelope>) -> IngestOutput {
    IngestOutput {
        events,
        coverage: CoverageManifest {
            run_id: RunId("bdd_run".into()),
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
        },
    }
}

fn test_engine() -> Engine<'static> {
    let renderer: &'static dyn shiplog_ports::Renderer =
        Box::leak(Box::new(MarkdownRenderer::default()));
    let clusterer: &'static dyn shiplog_ports::WorkstreamClusterer =
        Box::leak(Box::new(RepoClusterer));
    let redactor: &'static dyn shiplog_ports::Redactor =
        Box::leak(Box::new(DeterministicRedactor::new(b"bdd-test-key")));
    Engine::new(renderer, clusterer, redactor)
}

// ===========================================================================
// Scenario 1: Full pipeline produces all expected output files
// ===========================================================================

#[test]
fn scenario_full_pipeline_creates_all_artifacts() {
    Scenario::new("Full pipeline creates packet, ledger, coverage, bundle, and profile packets")
        .given("a set of PR events from two repos", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("bdd_run"));
            // Keep tempdir alive
            ctx.strings.insert(
                "tmp_root".into(),
                dir.keep().to_string_lossy().into_owned(),
            );
        })
        .when("the engine runs the full pipeline", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("acme/api", 1, "Add auth endpoint"),
                pr_event("acme/api", 2, "Fix rate limiting"),
                pr_event("acme/web", 3, "Redesign dashboard"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let (outputs, _ws_source) = engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            ctx.flags
                .insert("packet_exists".into(), outputs.packet_md.exists());
            ctx.flags.insert(
                "ledger_exists".into(),
                outputs.ledger_events_jsonl.exists(),
            );
            ctx.flags.insert(
                "coverage_exists".into(),
                outputs.coverage_manifest_json.exists(),
            );
            ctx.flags.insert(
                "bundle_exists".into(),
                outputs.bundle_manifest_json.exists(),
            );
            ctx.flags.insert(
                "manager_profile_exists".into(),
                out_dir
                    .join(DIR_PROFILES)
                    .join(PROFILE_MANAGER)
                    .join(FILE_PACKET_MD)
                    .exists(),
            );
            ctx.flags.insert(
                "public_profile_exists".into(),
                out_dir
                    .join(DIR_PROFILES)
                    .join(PROFILE_PUBLIC)
                    .join(FILE_PACKET_MD)
                    .exists(),
            );
            Ok(())
        })
        .then("packet.md exists", |ctx| {
            assert_true(ctx.flag("packet_exists").unwrap_or(false), "packet.md")
        })
        .then("ledger.events.jsonl exists", |ctx| {
            assert_true(
                ctx.flag("ledger_exists").unwrap_or(false),
                "ledger.events.jsonl",
            )
        })
        .then("coverage.manifest.json exists", |ctx| {
            assert_true(
                ctx.flag("coverage_exists").unwrap_or(false),
                "coverage.manifest.json",
            )
        })
        .then("bundle.manifest.json exists", |ctx| {
            assert_true(
                ctx.flag("bundle_exists").unwrap_or(false),
                "bundle.manifest.json",
            )
        })
        .then("manager profile packet exists", |ctx| {
            assert_true(
                ctx.flag("manager_profile_exists").unwrap_or(false),
                "manager profile packet",
            )
        })
        .then("public profile packet exists", |ctx| {
            assert_true(
                ctx.flag("public_profile_exists").unwrap_or(false),
                "public profile packet",
            )
        })
        .run()
        .expect("full pipeline scenario should pass");
}

// ===========================================================================
// Scenario 2: Pipeline clusters events by repository
// ===========================================================================

#[test]
fn scenario_pipeline_clusters_events_by_repo() {
    Scenario::new("Pipeline clusters events into repo-based workstreams")
        .given("events from three different repositories", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("bdd_cluster"));
            ctx.strings.insert(
                "tmp_root".into(),
                dir.keep().to_string_lossy().into_owned(),
            );
        })
        .when("the engine runs and generates workstreams", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("org/backend", 1, "Backend change 1"),
                pr_event("org/backend", 2, "Backend change 2"),
                pr_event("org/frontend", 3, "Frontend update"),
                pr_event("org/infra", 4, "Infra setup"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let (outputs, ws_source) = engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            // Read packet to verify workstream sections
            let packet = std::fs::read_to_string(&outputs.packet_md)
                .map_err(|e| e.to_string())?;
            ctx.flags
                .insert("has_backend".into(), packet.contains("org/backend"));
            ctx.flags
                .insert("has_frontend".into(), packet.contains("org/frontend"));
            ctx.flags
                .insert("has_infra".into(), packet.contains("org/infra"));
            ctx.flags.insert(
                "ws_generated".into(),
                matches!(ws_source, WorkstreamSource::Generated),
            );
            Ok(())
        })
        .then("packet mentions the backend workstream", |ctx| {
            assert_true(
                ctx.flag("has_backend").unwrap_or(false),
                "backend workstream in packet",
            )
        })
        .then("packet mentions the frontend workstream", |ctx| {
            assert_true(
                ctx.flag("has_frontend").unwrap_or(false),
                "frontend workstream in packet",
            )
        })
        .then("packet mentions the infra workstream", |ctx| {
            assert_true(
                ctx.flag("has_infra").unwrap_or(false),
                "infra workstream in packet",
            )
        })
        .then("workstream source is Generated", |ctx| {
            assert_true(
                ctx.flag("ws_generated").unwrap_or(false),
                "workstream source should be Generated",
            )
        })
        .run()
        .expect("clustering scenario should pass");
}

// ===========================================================================
// Scenario 3: Public profile packet does not contain sensitive data
// ===========================================================================

#[test]
fn scenario_public_profile_packet_strips_sensitive_data() {
    Scenario::new("Public profile packet does not expose PR titles or repo names")
        .given("events with sensitive PR titles", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("bdd_public"));
            ctx.strings.insert(
                "tmp_root".into(),
                dir.keep().to_string_lossy().into_owned(),
            );
            ctx.strings
                .insert("sensitive_title".into(), "Fix secret API key leak".into());
            ctx.strings
                .insert("sensitive_repo".into(), "acme/secret-service".into());
        })
        .when("the engine produces output including profiles", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let title = ctx.string("sensitive_title").unwrap().to_string();
            let repo = ctx.string("sensitive_repo").unwrap().to_string();
            let events = vec![pr_event(&repo, 1, &title)];
            let ingest = test_ingest(events);
            let engine = test_engine();

            engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            let public_packet_path = out_dir
                .join(DIR_PROFILES)
                .join(PROFILE_PUBLIC)
                .join(FILE_PACKET_MD);
            let public_packet =
                std::fs::read_to_string(&public_packet_path).map_err(|e| e.to_string())?;
            ctx.strings
                .insert("public_packet".into(), public_packet);
            Ok(())
        })
        .then("the public packet does not contain the sensitive title", |ctx| {
            let packet = ctx.string("public_packet").unwrap();
            if packet.contains("Fix secret API key leak") {
                return Err("public packet should not contain sensitive PR title".into());
            }
            Ok(())
        })
        .then("the public packet does not contain the repo name", |ctx| {
            let packet = ctx.string("public_packet").unwrap();
            if packet.contains("acme/secret-service") {
                return Err("public packet should not contain repo name".into());
            }
            Ok(())
        })
        .run()
        .expect("public profile redaction scenario should pass");
}

// ===========================================================================
// Scenario 4: Pipeline with zip produces archive
// ===========================================================================

#[test]
fn scenario_pipeline_with_zip_creates_archive() {
    Scenario::new("Pipeline with zip=true creates a zip archive")
        .given("events and zip mode enabled", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("bdd_zip"));
            ctx.strings.insert(
                "tmp_root".into(),
                dir.keep().to_string_lossy().into_owned(),
            );
        })
        .when("the engine runs with zip enabled", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![pr_event("acme/app", 1, "Feature")];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let (outputs, _) = engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    true,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            ctx.flags
                .insert("has_zip".into(), outputs.zip_path.is_some());
            if let Some(ref zp) = outputs.zip_path {
                ctx.flags.insert("zip_exists".into(), zp.exists());
            }
            Ok(())
        })
        .then("zip_path is Some", |ctx| {
            assert_true(
                ctx.flag("has_zip").unwrap_or(false),
                "zip_path should be Some",
            )
        })
        .then("the zip file exists on disk", |ctx| {
            assert_true(
                ctx.flag("zip_exists").unwrap_or(false),
                "zip file should exist",
            )
        })
        .run()
        .expect("zip scenario should pass");
}
