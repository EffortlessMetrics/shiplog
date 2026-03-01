//! BDD workflow integration tests for end-to-end user workflows.
//!
//! Each scenario exercises a complete user workflow through the engine,
//! using real APIs and file I/O to verify behavior.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_engine::Engine;
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_output_layout::{DIR_PROFILES, FILE_PACKET_MD, PROFILE_PUBLIC};
use shiplog_ports::IngestOutput;
use shiplog_redact::DeterministicRedactor;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::bundle::BundleProfile;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use shiplog_testkit::bdd::Scenario;
use shiplog_testkit::bdd::assertions::*;
use shiplog_workstream_cluster::RepoClusterer;
use shiplog_workstream_layout::WorkstreamManager;

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
            run_id: shiplog_ids::RunId("bdd_workflow_run".into()),
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
        Box::leak(Box::new(DeterministicRedactor::new(b"bdd-workflow-key")));
    Engine::new(renderer, clusterer, redactor)
}

fn make_curated_workstreams(
    title: &str,
    summary: &str,
    event_ids: Vec<EventId>,
) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", &title.to_lowercase().replace(' ', "-")]),
            title: title.to_string(),
            summary: Some(summary.to_string()),
            tags: vec!["curated".into()],
            stats: WorkstreamStats {
                pull_requests: event_ids.len(),
                reviews: 0,
                manual_events: 0,
            },
            events: event_ids,
            receipts: vec![],
        }],
    }
}

// ===========================================================================
// Scenario 1: Full pipeline workflow
// Given events from multiple repos -> When engine processes -> Then coverage
// manifest is complete
// ===========================================================================

#[test]
fn scenario_full_pipeline_multi_repo_coverage_complete() {
    Scenario::new("Events from multiple repos produce complete coverage manifest")
        .given("PR events from three distinct repositories", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("full_pipeline"));
            ctx.strings
                .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());
        })
        .when("the engine processes all events", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("acme/api", 1, "Add auth endpoint"),
                pr_event("acme/api", 2, "Fix rate limiting"),
                pr_event("acme/web", 3, "Redesign dashboard"),
                pr_event("acme/web", 4, "Add dark mode"),
                pr_event("acme/infra", 5, "Setup CI/CD"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let (outputs, _) = engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            // Parse coverage manifest from disk
            let coverage_text = std::fs::read_to_string(&outputs.coverage_manifest_json)
                .map_err(|e| e.to_string())?;
            let coverage: CoverageManifest =
                serde_json::from_str(&coverage_text).map_err(|e| e.to_string())?;

            ctx.strings.insert(
                "completeness".into(),
                format!("{:?}", coverage.completeness),
            );
            ctx.flags.insert(
                "coverage_exists".into(),
                outputs.coverage_manifest_json.exists(),
            );
            ctx.flags
                .insert("packet_exists".into(), outputs.packet_md.exists());
            ctx.flags
                .insert("ledger_exists".into(), outputs.ledger_events_jsonl.exists());

            // Count events in ledger
            let ledger_text =
                std::fs::read_to_string(&outputs.ledger_events_jsonl).map_err(|e| e.to_string())?;
            ctx.numbers.insert(
                "ledger_event_count".into(),
                ledger_text.lines().count() as u64,
            );

            // Verify packet mentions all repos
            let packet = std::fs::read_to_string(&outputs.packet_md).map_err(|e| e.to_string())?;
            ctx.flags
                .insert("has_api_repo".into(), packet.contains("acme/api"));
            ctx.flags
                .insert("has_web_repo".into(), packet.contains("acme/web"));
            ctx.flags
                .insert("has_infra_repo".into(), packet.contains("acme/infra"));

            Ok(())
        })
        .then("the coverage manifest reports Complete", |ctx| {
            let completeness = assert_present(ctx.string("completeness"), "completeness")?;
            assert_eq(completeness, "Complete", "coverage completeness")
        })
        .then("the ledger contains all five events", |ctx| {
            let count = assert_present(ctx.number("ledger_event_count"), "ledger event count")?;
            assert_eq(count, 5, "ledger event count")
        })
        .then("all output artifacts exist", |ctx| {
            assert_true(ctx.flag("packet_exists").unwrap_or(false), "packet.md")?;
            assert_true(
                ctx.flag("ledger_exists").unwrap_or(false),
                "ledger.events.jsonl",
            )?;
            assert_true(
                ctx.flag("coverage_exists").unwrap_or(false),
                "coverage.manifest.json",
            )
        })
        .then("the packet references all three repositories", |ctx| {
            assert_true(
                ctx.flag("has_api_repo").unwrap_or(false),
                "acme/api in packet",
            )?;
            assert_true(
                ctx.flag("has_web_repo").unwrap_or(false),
                "acme/web in packet",
            )?;
            assert_true(
                ctx.flag("has_infra_repo").unwrap_or(false),
                "acme/infra in packet",
            )
        })
        .run()
        .expect("full pipeline workflow should pass");
}

// ===========================================================================
// Scenario 2: Workstream curation workflow
// Given suggested workstreams -> When user edits -> Then re-render preserves
// edits
// ===========================================================================

#[test]
fn scenario_workstream_curation_preserved_on_rerender() {
    Scenario::new("User-curated workstreams survive re-render via refresh")
        .given(
            "an initial engine run that generates suggested workstreams",
            |ctx| {
                let dir = tempfile::tempdir().unwrap();
                let out_dir = dir.path().join("curation_workflow");
                ctx.paths.insert("out_dir".into(), out_dir.clone());
                ctx.strings
                    .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());

                // Run engine once to generate initial artifacts
                let events = vec![
                    pr_event("acme/api", 1, "Add auth"),
                    pr_event("acme/api", 2, "Fix bug"),
                ];
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
                    .unwrap();
            },
        )
        .given(
            "the user writes curated workstreams with a custom title",
            |ctx| {
                let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
                let event_ids = vec![
                    EventId::from_parts(["github", "pr", "acme/api", "1"]),
                    EventId::from_parts(["github", "pr", "acme/api", "2"]),
                ];
                let curated = make_curated_workstreams(
                    "Authentication Service Revamp",
                    "Major auth overhaul with OAuth2 support",
                    event_ids,
                );
                let curated_path = WorkstreamManager::curated_path(&out_dir);
                shiplog_workstream_layout::write_workstreams(&curated_path, &curated).unwrap();
                ctx.strings.insert(
                    "curated_title".into(),
                    "Authentication Service Revamp".into(),
                );
            },
        )
        .when("the engine refreshes with new events", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("acme/api", 1, "Add auth"),
                pr_event("acme/api", 2, "Fix bug"),
                pr_event("acme/api", 3, "Add token refresh"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            engine
                .refresh(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            // Read the packet to check if curated title survives
            let packet_path = out_dir.join("packet.md");
            let packet = std::fs::read_to_string(&packet_path).map_err(|e| e.to_string())?;
            ctx.strings.insert("packet_content".into(), packet);

            // Verify curated workstreams.yaml still has the custom title
            let curated_path = WorkstreamManager::curated_path(&out_dir);
            let curated_text = std::fs::read_to_string(&curated_path).map_err(|e| e.to_string())?;
            ctx.flags.insert(
                "curated_title_preserved".into(),
                curated_text.contains("Authentication Service Revamp"),
            );

            Ok(())
        })
        .then(
            "the curated workstream title is preserved in the file",
            |ctx| {
                assert_true(
                    ctx.flag("curated_title_preserved").unwrap_or(false),
                    "curated title in workstreams.yaml",
                )
            },
        )
        .then("the packet references the curated title", |ctx| {
            let packet = assert_present(ctx.string("packet_content"), "packet content")?;
            assert_contains(
                packet,
                "Authentication Service Revamp",
                "curated title in packet",
            )
        })
        .run()
        .expect("workstream curation workflow should pass");
}

// ===========================================================================
// Scenario 3: Redaction workflow
// Given internal packet -> When redacted with key -> Then public packet
// contains no PII
// ===========================================================================

#[test]
fn scenario_redaction_strips_pii_from_public_packet() {
    Scenario::new("Public packet contains no PII after redaction")
        .given(
            "events with sensitive PR titles, repo names, and URLs",
            |ctx| {
                let dir = tempfile::tempdir().unwrap();
                ctx.paths
                    .insert("out_dir".into(), dir.path().join("redaction_workflow"));
                ctx.strings
                    .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());
                ctx.strings.insert(
                    "sensitive_title".into(),
                    "Fix CVE-2025-9999 in payment-gateway".into(),
                );
                ctx.strings
                    .insert("sensitive_repo".into(), "acme/payment-gateway".into());
                ctx.strings.insert(
                    "sensitive_url".into(),
                    "https://github.com/acme/payment-gateway/pull/42".into(),
                );
            },
        )
        .when("the engine produces all profiles", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let title = ctx.string("sensitive_title").unwrap().to_string();
            let repo = ctx.string("sensitive_repo").unwrap().to_string();
            let events = vec![pr_event(&repo, 42, &title)];
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

            // Read internal and public packets
            let internal_packet =
                std::fs::read_to_string(out_dir.join("packet.md")).map_err(|e| e.to_string())?;
            let public_packet_path = out_dir
                .join(DIR_PROFILES)
                .join(PROFILE_PUBLIC)
                .join(FILE_PACKET_MD);
            let public_packet =
                std::fs::read_to_string(&public_packet_path).map_err(|e| e.to_string())?;

            ctx.strings
                .insert("internal_packet".into(), internal_packet);
            ctx.strings.insert("public_packet".into(), public_packet);
            Ok(())
        })
        .then("the internal packet contains the sensitive title", |ctx| {
            let packet = assert_present(ctx.string("internal_packet"), "internal packet")?;
            assert_contains(
                packet,
                "Fix CVE-2025-9999 in payment-gateway",
                "sensitive title in internal packet",
            )
        })
        .then(
            "the public packet does NOT contain the sensitive title",
            |ctx| {
                let packet = assert_present(ctx.string("public_packet"), "public packet")?;
                assert_not_contains(packet, "CVE-2025-9999", "CVE in public packet")?;
                assert_not_contains(
                    packet,
                    "Fix CVE-2025-9999 in payment-gateway",
                    "full sensitive title in public packet",
                )
            },
        )
        .then("the public packet does NOT contain the repo name", |ctx| {
            let packet = assert_present(ctx.string("public_packet"), "public packet")?;
            assert_not_contains(packet, "payment-gateway", "repo name in public packet")
        })
        .then("the public packet does NOT contain GitHub URLs", |ctx| {
            let packet = assert_present(ctx.string("public_packet"), "public packet")?;
            assert_not_contains(
                packet,
                "github.com/acme/payment-gateway",
                "GitHub URL in public packet",
            )
        })
        .run()
        .expect("redaction workflow should pass");
}

// ===========================================================================
// Scenario 5: Refresh workflow
// Given existing run -> When refresh called -> Then events updated,
// workstreams preserved
// ===========================================================================

#[test]
fn scenario_refresh_updates_events_preserves_workstreams() {
    Scenario::new("Refresh updates ledger events while preserving curated workstreams")
        .given("an existing run with curated workstreams", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            let out_dir = dir.path().join("refresh_workflow");
            ctx.paths.insert("out_dir".into(), out_dir.clone());
            ctx.strings
                .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());

            // Initial run
            let events = vec![pr_event("acme/app", 1, "Initial feature")];
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
                .unwrap();

            // Write curated workstreams
            let curated = make_curated_workstreams(
                "Core App Development",
                "Main application features",
                vec![EventId::from_parts(["github", "pr", "acme/app", "1"])],
            );
            let curated_path = WorkstreamManager::curated_path(&out_dir);
            shiplog_workstream_layout::write_workstreams(&curated_path, &curated).unwrap();
        })
        .when("refresh is called with additional events", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("acme/app", 1, "Initial feature"),
                pr_event("acme/app", 2, "Second feature"),
                pr_event("acme/app", 3, "Third feature"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let outputs = engine
                .refresh(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            // Count ledger events
            let ledger_text =
                std::fs::read_to_string(&outputs.ledger_events_jsonl).map_err(|e| e.to_string())?;
            ctx.numbers.insert(
                "ledger_event_count".into(),
                ledger_text.lines().count() as u64,
            );

            // Check curated workstreams file still exists and has correct title
            let curated_path = WorkstreamManager::curated_path(&out_dir);
            let curated_text = std::fs::read_to_string(&curated_path).map_err(|e| e.to_string())?;
            ctx.flags.insert(
                "curated_preserved".into(),
                curated_text.contains("Core App Development"),
            );

            // Verify packet references curated title
            let packet = std::fs::read_to_string(&outputs.packet_md).map_err(|e| e.to_string())?;
            ctx.flags.insert(
                "packet_has_curated_title".into(),
                packet.contains("Core App Development"),
            );

            Ok(())
        })
        .then("the ledger now contains three events", |ctx| {
            let count = assert_present(ctx.number("ledger_event_count"), "ledger event count")?;
            assert_eq(count, 3, "refreshed ledger event count")
        })
        .then("the curated workstream title is preserved", |ctx| {
            assert_true(
                ctx.flag("curated_preserved").unwrap_or(false),
                "curated workstream title in file",
            )
        })
        .then("the packet uses the curated workstream title", |ctx| {
            assert_true(
                ctx.flag("packet_has_curated_title").unwrap_or(false),
                "curated title in rendered packet",
            )
        })
        .run()
        .expect("refresh workflow should pass");
}

// ===========================================================================
// Scenario 6: Multi-format output
// Given processed events -> When rendered -> Then Markdown and JSON both valid
// ===========================================================================

#[test]
fn scenario_multi_format_output_both_valid() {
    Scenario::new("Engine produces valid Markdown and JSON outputs")
        .given("a set of PR events", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            ctx.paths
                .insert("out_dir".into(), dir.path().join("multi_format"));
            ctx.strings
                .insert("tmp_root".into(), dir.keep().to_string_lossy().into_owned());
        })
        .when("the engine renders all output formats", |ctx| {
            let out_dir = ctx.path("out_dir").unwrap().to_path_buf();
            let events = vec![
                pr_event("acme/api", 1, "Add endpoint"),
                pr_event("acme/web", 2, "Fix layout"),
            ];
            let ingest = test_ingest(events);
            let engine = test_engine();

            let (outputs, _) = engine
                .run(
                    ingest,
                    "tester",
                    "2025-01-01..2025-02-01",
                    &out_dir,
                    false,
                    &BundleProfile::Internal,
                )
                .map_err(|e| e.to_string())?;

            // Validate Markdown
            let packet = std::fs::read_to_string(&outputs.packet_md).map_err(|e| e.to_string())?;
            ctx.flags
                .insert("md_has_heading".into(), packet.starts_with('#'));
            ctx.flags.insert("md_non_empty".into(), !packet.is_empty());

            // Validate JSONL: each line must parse as an EventEnvelope
            let ledger_text =
                std::fs::read_to_string(&outputs.ledger_events_jsonl).map_err(|e| e.to_string())?;
            let mut jsonl_valid = true;
            let mut jsonl_count = 0u64;
            for line in ledger_text.lines() {
                if serde_json::from_str::<EventEnvelope>(line).is_err() {
                    jsonl_valid = false;
                    break;
                }
                jsonl_count += 1;
            }
            ctx.flags.insert("jsonl_valid".into(), jsonl_valid);
            ctx.numbers.insert("jsonl_event_count".into(), jsonl_count);

            // Validate coverage manifest JSON
            let cov_text = std::fs::read_to_string(&outputs.coverage_manifest_json)
                .map_err(|e| e.to_string())?;
            let cov_valid = serde_json::from_str::<CoverageManifest>(&cov_text).is_ok();
            ctx.flags.insert("coverage_json_valid".into(), cov_valid);

            Ok(())
        })
        .then(
            "the Markdown packet is non-empty and starts with a heading",
            |ctx| {
                assert_true(
                    ctx.flag("md_non_empty").unwrap_or(false),
                    "Markdown non-empty",
                )?;
                assert_true(
                    ctx.flag("md_has_heading").unwrap_or(false),
                    "Markdown starts with heading",
                )
            },
        )
        .then("every JSONL line parses as a valid EventEnvelope", |ctx| {
            assert_true(
                ctx.flag("jsonl_valid").unwrap_or(false),
                "all JSONL lines valid",
            )
        })
        .then("the JSONL event count matches the input", |ctx| {
            let count = assert_present(ctx.number("jsonl_event_count"), "JSONL event count")?;
            assert_eq(count, 2, "JSONL event count matches input")
        })
        .then("the coverage manifest is valid JSON", |ctx| {
            assert_true(
                ctx.flag("coverage_json_valid").unwrap_or(false),
                "coverage manifest JSON valid",
            )
        })
        .run()
        .expect("multi-format output workflow should pass");
}
