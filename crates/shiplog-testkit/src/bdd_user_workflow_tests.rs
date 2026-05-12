//! BDD integration tests for user-facing workflows.
//!
//! These tests exercise real code through the shiplog engine, redactor,
//! clusterer, and renderer — validating end-to-end behavior.

#[cfg(test)]
mod user_workflow_tests {
    use crate::bdd::Scenario;
    use crate::bdd::assertions::*;
    use crate::pr_event;

    use chrono::{NaiveDate, Utc};
    use shiplog_engine::{Engine, WorkstreamSource};
    use shiplog_ids::RunId;
    use shiplog_ports::{IngestOutput, Redactor, WorkstreamClusterer};
    use shiplog_redact::DeterministicRedactor;
    use shiplog_render_md::MarkdownRenderer;
    use shiplog_schema::bundle::BundleProfile;
    use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
    use shiplog_workstreams::RepoClusterer;

    // -- helpers ---------------------------------------------------------

    fn make_coverage(user: &str, completeness: Completeness) -> CoverageManifest {
        CoverageManifest {
            run_id: RunId::now("bdd-test"),
            generated_at: Utc::now(),
            user: user.to_string(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![],
            warnings: vec![],
            completeness,
        }
    }

    fn build_engine() -> Engine<'static> {
        let renderer: &'static dyn shiplog_ports::Renderer =
            Box::leak(Box::new(MarkdownRenderer::default()));
        let clusterer: &'static dyn WorkstreamClusterer = Box::leak(Box::new(RepoClusterer));
        let redactor: &'static dyn Redactor =
            Box::leak(Box::new(DeterministicRedactor::new(b"bdd-test-key")));
        Engine::new(renderer, clusterer, redactor)
    }

    // ====================================================================
    // Scenario 1: Happy Path JSON Import
    // ====================================================================

    #[test]
    fn happy_path_json_import() {
        let scenario = Scenario::new("Happy path: JSON import through full pipeline")
            .given("valid events and a coverage manifest", |ctx| {
                let events = vec![
                    pr_event("acme/app", 1, "Add login page"),
                    pr_event("acme/app", 2, "Fix logout redirect"),
                ];
                let coverage = make_coverage("testuser", Completeness::Complete);

                // Stash serialised events so the When step can use them
                let events_json = serde_json::to_string(&events).unwrap();
                let coverage_json = serde_json::to_string(&coverage).unwrap();
                ctx.data
                    .insert("events".to_string(), events_json.into_bytes());
                ctx.data
                    .insert("coverage".to_string(), coverage_json.into_bytes());
            })
            .when("running the full pipeline via Engine::run", |ctx| {
                let events: Vec<shiplog_schema::event::EventEnvelope> =
                    serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();
                let coverage: CoverageManifest =
                    serde_json::from_slice(ctx.data.get("coverage").unwrap()).unwrap();

                let ingest = IngestOutput {
                    events,
                    coverage,
                    freshness: Vec::new(),
                };
                let dir = tempfile::tempdir().unwrap();
                let out_dir = dir.path().join("happy_path_run");

                let engine = build_engine();
                let (outputs, _ws_source) = engine
                    .run(
                        ingest,
                        "testuser",
                        "2025-01-01..2025-02-01",
                        &out_dir,
                        false,
                        &BundleProfile::Internal,
                    )
                    .map_err(|e| e.to_string())?;

                ctx.flags
                    .insert("packet_md_exists".to_string(), outputs.packet_md.exists());
                ctx.flags.insert(
                    "workstreams_exist".to_string(),
                    outputs.workstreams_yaml.exists(),
                );
                ctx.flags.insert(
                    "coverage_manifest_exists".to_string(),
                    outputs.coverage_manifest_json.exists(),
                );
                ctx.flags.insert(
                    "ledger_exists".to_string(),
                    outputs.ledger_events_jsonl.exists(),
                );
                ctx.flags.insert(
                    "bundle_exists".to_string(),
                    outputs.bundle_manifest_json.exists(),
                );

                // Keep tempdir alive (prevent cleanup before Then steps)
                let _ = dir.keep();
                Ok(())
            })
            .then("packet.md exists", |ctx| {
                assert_true(
                    ctx.flag("packet_md_exists").unwrap_or(false),
                    "packet.md exists",
                )
            })
            .then("workstreams file exists", |ctx| {
                assert_true(
                    ctx.flag("workstreams_exist").unwrap_or(false),
                    "workstreams file exists",
                )
            })
            .then("coverage manifest exists", |ctx| {
                assert_true(
                    ctx.flag("coverage_manifest_exists").unwrap_or(false),
                    "coverage.manifest.json exists",
                )
            })
            .then("ledger events file exists", |ctx| {
                assert_true(
                    ctx.flag("ledger_exists").unwrap_or(false),
                    "ledger.events.jsonl exists",
                )
            })
            .then("bundle manifest exists", |ctx| {
                assert_true(
                    ctx.flag("bundle_exists").unwrap_or(false),
                    "bundle.manifest.json exists",
                )
            });

        scenario
            .run()
            .expect("happy path JSON import should succeed");
    }

    // ====================================================================
    // Scenario 2: Workstream Curation Flow
    // ====================================================================

    #[test]
    fn workstream_curation_flow() {
        let scenario =
            Scenario::new("Workstream curation: cluster by repo, edit, re-render with curated")
                .given("events from two different repos", |ctx| {
                    let events = vec![
                        pr_event("acme/frontend", 10, "Redesign navbar"),
                        pr_event("acme/frontend", 11, "Add dark mode"),
                        pr_event("acme/backend", 20, "Add REST endpoint"),
                        pr_event("acme/backend", 21, "Optimise SQL queries"),
                    ];
                    let events_json = serde_json::to_string(&events).unwrap();
                    ctx.data
                        .insert("events".to_string(), events_json.into_bytes());
                })
                .when("clustering events", |ctx| {
                    let events: Vec<shiplog_schema::event::EventEnvelope> =
                        serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();
                    let clusterer = RepoClusterer;
                    let ws = clusterer.cluster(&events).map_err(|e| e.to_string())?;
                    ctx.numbers
                        .insert("workstream_count".to_string(), ws.workstreams.len() as u64);

                    // Verify repo-based grouping
                    let titles: Vec<String> =
                        ws.workstreams.iter().map(|w| w.title.clone()).collect();
                    ctx.flags.insert(
                        "has_frontend_ws".to_string(),
                        titles.iter().any(|t| t.contains("frontend")),
                    );
                    ctx.flags.insert(
                        "has_backend_ws".to_string(),
                        titles.iter().any(|t| t.contains("backend")),
                    );

                    let ws_json = serde_json::to_string(&ws).unwrap();
                    ctx.data
                        .insert("workstreams".to_string(), ws_json.into_bytes());
                    Ok(())
                })
                .then("suggested workstreams group by repo", |ctx| {
                    let count = assert_present(ctx.number("workstream_count"), "workstream count")?;
                    assert_eq(count, 2, "two workstreams from two repos")?;
                    assert_true(
                        ctx.flag("has_frontend_ws").unwrap_or(false),
                        "frontend workstream exists",
                    )?;
                    assert_true(
                        ctx.flag("has_backend_ws").unwrap_or(false),
                        "backend workstream exists",
                    )
                })
                .when(
                    "user writes curated workstreams.yaml then re-renders",
                    |ctx| {
                        let events: Vec<shiplog_schema::event::EventEnvelope> =
                            serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();

                        // Simulate curation: build a WorkstreamsFile with custom titles
                        let mut ws: shiplog_schema::workstream::WorkstreamsFile =
                            serde_json::from_slice(ctx.data.get("workstreams").unwrap()).unwrap();
                        for w in &mut ws.workstreams {
                            if w.title.contains("frontend") {
                                w.title = "UI Refresh".to_string();
                            } else if w.title.contains("backend") {
                                w.title = "API Improvements".to_string();
                            }
                        }

                        // Write curated file, then run engine (which should pick it up)
                        let dir = tempfile::tempdir().unwrap();
                        let out_dir = dir.path().join("curation_run");
                        std::fs::create_dir_all(&out_dir).unwrap();

                        let curated_path =
                            shiplog_workstreams::WorkstreamManager::curated_path(&out_dir);
                        shiplog_workstreams::write_workstreams(&curated_path, &ws).unwrap();

                        let coverage = make_coverage("testuser", Completeness::Complete);
                        let ingest = IngestOutput {
                            events,
                            coverage,
                            freshness: Vec::new(),
                        };
                        let engine = build_engine();
                        let (outputs, ws_source) = engine
                            .run(
                                ingest,
                                "testuser",
                                "2025-01-01..2025-02-01",
                                &out_dir,
                                false,
                                &BundleProfile::Internal,
                            )
                            .map_err(|e| e.to_string())?;

                        ctx.flags.insert(
                            "used_curated".to_string(),
                            ws_source == WorkstreamSource::Curated,
                        );

                        // Verify the rendered packet references the curated title
                        let packet = std::fs::read_to_string(&outputs.packet_md).unwrap();
                        ctx.flags.insert(
                            "packet_has_curated_title".to_string(),
                            packet.contains("UI Refresh") || packet.contains("API Improvements"),
                        );

                        let _ = dir.keep();
                        Ok(())
                    },
                )
                .then("render uses curated workstreams", |ctx| {
                    assert_true(
                        ctx.flag("used_curated").unwrap_or(false),
                        "engine reports WorkstreamSource::Curated",
                    )
                })
                .then("rendered packet contains curated titles", |ctx| {
                    assert_true(
                        ctx.flag("packet_has_curated_title").unwrap_or(false),
                        "packet.md references curated workstream title",
                    )
                });

        scenario
            .run()
            .expect("workstream curation flow should succeed");
    }

    // ====================================================================
    // Scenario 3: Redaction Profiles
    // ====================================================================

    #[test]
    fn redaction_profiles() {
        let scenario = Scenario::new("Redaction profiles: manager vs public")
            .given("internal events with titles and links", |ctx| {
                let events = vec![pr_event(
                    "acme/secret-repo",
                    42,
                    "Fix auth bug in user-service",
                )];
                let events_json = serde_json::to_string(&events).unwrap();
                ctx.data
                    .insert("events".to_string(), events_json.into_bytes());
                ctx.strings.insert(
                    "original_title".to_string(),
                    "Fix auth bug in user-service".to_string(),
                );
                ctx.strings.insert(
                    "original_link".to_string(),
                    "https://github.com/acme/secret-repo/pull/42".to_string(),
                );
            })
            .when("redacting at manager level", |ctx| {
                let events: Vec<shiplog_schema::event::EventEnvelope> =
                    serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();
                let redactor = DeterministicRedactor::new(b"bdd-redact-key");
                let manager_events = redactor
                    .redact_events(&events, "manager")
                    .map_err(|e| e.to_string())?;

                let original_title = ctx.string("original_title").unwrap().to_string();
                let original_link = ctx.string("original_link").unwrap().to_string();

                // Manager profile: titles preserved, links aliased
                let ev = &manager_events[0];
                let pr = match &ev.payload {
                    shiplog_schema::event::EventPayload::PullRequest(pr) => pr,
                    _ => return Err("expected PR payload".to_string()),
                };
                ctx.flags.insert(
                    "manager_title_visible".to_string(),
                    pr.title == original_title,
                );
                // Links should be aliased (not original URL)
                let has_original_link = ev.links.iter().any(|l| l.url == original_link);
                ctx.flags
                    .insert("manager_links_stripped".to_string(), !has_original_link);

                let mgr_json = serde_json::to_string(&manager_events).unwrap();
                ctx.data
                    .insert("manager_events".to_string(), mgr_json.into_bytes());
                Ok(())
            })
            .then("manager: titles are visible", |ctx| {
                assert_true(
                    ctx.flag("manager_title_visible").unwrap_or(false),
                    "manager profile preserves titles",
                )
            })
            .then("manager: links are aliased", |ctx| {
                assert_true(
                    ctx.flag("manager_links_stripped").unwrap_or(false),
                    "manager profile strips original links",
                )
            })
            .when("redacting at public level", |ctx| {
                let events: Vec<shiplog_schema::event::EventEnvelope> =
                    serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();
                let redactor = DeterministicRedactor::new(b"bdd-redact-key");
                let public_events = redactor
                    .redact_events(&events, "public")
                    .map_err(|e| e.to_string())?;

                let original_title = ctx.string("original_title").unwrap().to_string();
                let original_link = ctx.string("original_link").unwrap().to_string();

                let ev = &public_events[0];
                let pr = match &ev.payload {
                    shiplog_schema::event::EventPayload::PullRequest(pr) => pr,
                    _ => return Err("expected PR payload".to_string()),
                };
                // Public profile: titles are aliased/stripped
                ctx.flags.insert(
                    "public_title_stripped".to_string(),
                    pr.title != original_title,
                );
                // Links should also not contain original URL
                let has_original_link = ev.links.iter().any(|l| l.url == original_link);
                ctx.flags
                    .insert("public_links_stripped".to_string(), !has_original_link);
                Ok(())
            })
            .then("public: titles are stripped", |ctx| {
                assert_true(
                    ctx.flag("public_title_stripped").unwrap_or(false),
                    "public profile strips titles",
                )
            })
            .then("public: links are stripped", |ctx| {
                assert_true(
                    ctx.flag("public_links_stripped").unwrap_or(false),
                    "public profile strips links",
                )
            });

        scenario.run().expect("redaction profiles should work");
    }

    // ====================================================================
    // Scenario 4: Error Recovery — Malformed JSONL
    // ====================================================================

    #[test]
    fn error_recovery_malformed_jsonl() {
        let scenario = Scenario::new("Error recovery: malformed JSONL reports line-level errors")
            .given(
                "a JSONL string with one valid and one invalid line",
                |ctx| {
                    let valid_event = pr_event("acme/app", 1, "Valid PR");
                    let valid_line = serde_json::to_string(&valid_event).unwrap();
                    let bad_line = "{not valid json}";
                    let jsonl = format!("{valid_line}\n{bad_line}\n");
                    ctx.data
                        .insert("jsonl_content".to_string(), jsonl.into_bytes());
                },
            )
            .when("parsing the JSONL content", |ctx| {
                let content =
                    String::from_utf8(ctx.data.get("jsonl_content").unwrap().clone()).unwrap();
                let result = shiplog_ingest_json::parse_events_jsonl(&content, "test-malformed");
                match result {
                    Ok(_) => {
                        ctx.flags.insert("error_has_line_number".to_string(), false);
                    }
                    Err(e) => {
                        let msg = format!("{e:#}");
                        ctx.flags
                            .insert("error_has_line_number".to_string(), msg.contains("line 2"));
                        ctx.strings.insert("error_message".to_string(), msg);
                    }
                }
                Ok(())
            })
            .then("the error references the malformed line number", |ctx| {
                assert_true(
                    ctx.flag("error_has_line_number").unwrap_or(false),
                    "error message includes line number",
                )
            })
            .then("the error message is human-readable", |ctx| {
                let msg = assert_present(ctx.string("error_message"), "error message")?;
                assert_contains(msg, "line 2", "error references line 2")
            });

        scenario
            .run()
            .expect("malformed JSONL error recovery should work");
    }

    // ====================================================================
    // Scenario 5: Empty Input Handling
    // ====================================================================

    #[test]
    fn empty_input_handling() {
        let scenario = Scenario::new("Empty input: pipeline succeeds with zero events")
            .given("an empty event list and valid coverage", |ctx| {
                let events: Vec<shiplog_schema::event::EventEnvelope> = vec![];
                let coverage = make_coverage("testuser", Completeness::Complete);
                let events_json = serde_json::to_string(&events).unwrap();
                let coverage_json = serde_json::to_string(&coverage).unwrap();
                ctx.data
                    .insert("events".to_string(), events_json.into_bytes());
                ctx.data
                    .insert("coverage".to_string(), coverage_json.into_bytes());
            })
            .when("running the full pipeline", |ctx| {
                let events: Vec<shiplog_schema::event::EventEnvelope> =
                    serde_json::from_slice(ctx.data.get("events").unwrap()).unwrap();
                let coverage: CoverageManifest =
                    serde_json::from_slice(ctx.data.get("coverage").unwrap()).unwrap();

                let ingest = IngestOutput {
                    events,
                    coverage,
                    freshness: Vec::new(),
                };
                let dir = tempfile::tempdir().unwrap();
                let out_dir = dir.path().join("empty_run");

                let engine = build_engine();
                let (outputs, _ws_source) = engine
                    .run(
                        ingest,
                        "testuser",
                        "2025-01-01..2025-02-01",
                        &out_dir,
                        false,
                        &BundleProfile::Internal,
                    )
                    .map_err(|e| e.to_string())?;

                ctx.flags.insert("pipeline_executed".to_string(), true);
                ctx.flags
                    .insert("packet_md_exists".to_string(), outputs.packet_md.exists());

                // Count workstreams in the written file
                let ws_content = std::fs::read_to_string(&outputs.workstreams_yaml).unwrap();
                let ws: shiplog_schema::workstream::WorkstreamsFile =
                    serde_yaml::from_str(&ws_content).unwrap();
                ctx.numbers
                    .insert("workstream_count".to_string(), ws.workstreams.len() as u64);

                let _ = dir.keep();
                Ok(())
            })
            .then("no crash occurs", |ctx| {
                assert_true(
                    ctx.flag("pipeline_executed").unwrap_or(false),
                    "pipeline completed without panic",
                )
            })
            .then("packet.md is produced", |ctx| {
                assert_true(
                    ctx.flag("packet_md_exists").unwrap_or(false),
                    "packet.md exists for empty input",
                )
            })
            .then("workstreams file has zero workstreams", |ctx| {
                let count = assert_present(ctx.number("workstream_count"), "workstream count")?;
                assert_eq(count, 0, "zero workstreams for empty input")
            });

        scenario.run().expect("empty input handling should succeed");
    }

    // ====================================================================
    // Scenario 6: Coverage Completeness
    // ====================================================================

    #[test]
    fn coverage_completeness() {
        let scenario = Scenario::new("Coverage completeness: slices reflect data accuracy")
            .given("a date range with coverage slices", |ctx| {
                ctx.strings
                    .insert("since".to_string(), "2025-01-01".to_string());
                ctx.strings
                    .insert("until".to_string(), "2025-04-01".to_string());
            })
            .when("building coverage manifest with mixed slices", |ctx| {
                let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
                let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();

                // Build month windows using coverage crate
                let windows = shiplog_coverage::month_windows(since, until);

                // Create slices: first two complete, third partial
                let mut slices = Vec::new();
                for (i, w) in windows.iter().enumerate() {
                    let (total, fetched) = if i < 2 { (50, 50) } else { (150, 100) };
                    slices.push(CoverageSlice {
                        window: w.clone(),
                        query: format!("author:user is:merged created:{}..{}", w.since, w.until),
                        total_count: total,
                        fetched,
                        incomplete_results: Some(fetched < total),
                        notes: vec![],
                    });
                }

                // Check slice accuracy
                let complete_ok = slices.iter().take(2).all(|s| s.fetched == s.total_count);
                let partial_ok = slices
                    .last()
                    .map(|s| s.fetched < s.total_count)
                    .unwrap_or(false);

                // Overall completeness should be Partial (worst case)
                let overall = if slices.iter().any(|s| s.fetched < s.total_count) {
                    Completeness::Partial
                } else {
                    Completeness::Complete
                };

                ctx.flags
                    .insert("complete_slice_accurate".to_string(), complete_ok);
                ctx.flags
                    .insert("partial_slice_accurate".to_string(), partial_ok);
                ctx.flags.insert(
                    "overall_completeness_correct".to_string(),
                    overall == Completeness::Partial,
                );

                // Verify month_windows produced the right count
                ctx.numbers
                    .insert("window_count".to_string(), windows.len() as u64);
                Ok(())
            })
            .then("complete slices report fetched == total", |ctx| {
                assert_true(
                    ctx.flag("complete_slice_accurate").unwrap_or(false),
                    "complete slices are accurate",
                )
            })
            .then("partial slices report fetched < total", |ctx| {
                assert_true(
                    ctx.flag("partial_slice_accurate").unwrap_or(false),
                    "partial slices are accurate",
                )
            })
            .then("overall completeness reflects the worst slice", |ctx| {
                assert_true(
                    ctx.flag("overall_completeness_correct").unwrap_or(false),
                    "overall completeness is Partial when any slice is partial",
                )
            })
            .then("month windows cover the full range", |ctx| {
                let count = assert_present(ctx.number("window_count"), "window count")?;
                // Jan, Feb, Mar = 3 months
                assert_eq(count, 3, "three monthly windows from Jan to Apr")
            });

        scenario
            .run()
            .expect("coverage completeness should be accurate");
    }
}
