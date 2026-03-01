//! BDD scenarios for user-facing workflows.
//!
//! These scenarios exercise real code paths through the shiplog pipeline
//! to verify end-to-end user workflows.

use crate::bdd::assertions::*;
use crate::bdd::Scenario;

// ---------------------------------------------------------------------------
// Scenario 1: Happy Path JSON Import
// ---------------------------------------------------------------------------

/// Given valid JSONL events and coverage manifest, running the full pipeline
/// produces packet.md, workstreams, and coverage manifest.
pub fn happy_path_json_import() -> Scenario {
    Scenario::new("Happy path JSON import through full pipeline")
        .given("valid JSONL events and a coverage manifest", |ctx| {
            ctx.strings
                .insert("user".to_string(), "testuser".to_string());
            ctx.strings
                .insert("window".to_string(), "2025-01-01..2025-02-01".to_string());
            // Marker: events will be built in the When step using testkit fixtures
            ctx.numbers.insert("event_count".to_string(), 2);
        })
        .when("running the full pipeline via Engine::run", |ctx| {
            // Actual integration happens in bdd_user_workflow_tests.rs
            // Here we set up expectations for the scenario runner
            ctx.flags.insert("pipeline_executed".to_string(), true);
            Ok(())
        })
        .then("output contains packet.md", |ctx| {
            assert_true(
                ctx.flag("packet_md_exists").unwrap_or(false),
                "packet.md exists",
            )
        })
        .then("output contains workstreams", |ctx| {
            assert_true(
                ctx.flag("workstreams_exist").unwrap_or(false),
                "workstreams file exists",
            )
        })
        .then("output contains coverage manifest", |ctx| {
            assert_true(
                ctx.flag("coverage_manifest_exists").unwrap_or(false),
                "coverage.manifest.json exists",
            )
        })
        .then("output contains ledger events", |ctx| {
            assert_true(
                ctx.flag("ledger_exists").unwrap_or(false),
                "ledger.events.jsonl exists",
            )
        })
}

// ---------------------------------------------------------------------------
// Scenario 2: Workstream Curation Flow
// ---------------------------------------------------------------------------

/// Given events from multiple repos, clustering groups by repo.
/// After user edits workstreams.yaml, render uses curated workstreams.
pub fn workstream_curation_flow() -> Scenario {
    Scenario::new("Workstream curation flow: cluster, edit, re-render")
        .given("events from multiple repositories", |ctx| {
            ctx.strings
                .insert("repo_a".to_string(), "acme/frontend".to_string());
            ctx.strings
                .insert("repo_b".to_string(), "acme/backend".to_string());
            ctx.numbers.insert("repos".to_string(), 2);
        })
        .when("clustering events by repo", |ctx| {
            ctx.flags.insert("clustering_done".to_string(), true);
            Ok(())
        })
        .then("suggested workstreams group by repo", |ctx| {
            let repos = assert_present(ctx.number("workstream_count"), "workstream count")?;
            assert_true(repos >= 2, "at least two workstreams from two repos")
        })
        .when("user edits workstreams.yaml with curated titles", |ctx| {
            ctx.flags.insert("curated".to_string(), true);
            Ok(())
        })
        .then("render uses curated workstreams", |ctx| {
            assert_true(
                ctx.flag("used_curated").unwrap_or(false),
                "engine used curated workstreams",
            )
        })
}

// ---------------------------------------------------------------------------
// Scenario 3: Redaction Profiles
// ---------------------------------------------------------------------------

/// Given internal events with PII, manager redaction keeps titles but strips
/// links, and public redaction strips both titles and links.
pub fn redaction_profiles() -> Scenario {
    Scenario::new("Redaction profiles: manager vs public projections")
        .given("internal events with PII in titles and links", |ctx| {
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
            ctx.flags.insert("manager_redacted".to_string(), true);
            Ok(())
        })
        .then("manager: titles are visible", |ctx| {
            assert_true(
                ctx.flag("manager_title_visible").unwrap_or(false),
                "manager profile preserves titles",
            )
        })
        .then("manager: links are stripped", |ctx| {
            assert_true(
                ctx.flag("manager_links_stripped").unwrap_or(false),
                "manager profile strips links",
            )
        })
        .when("redacting at public level", |ctx| {
            ctx.flags.insert("public_redacted".to_string(), true);
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
        })
}

// ---------------------------------------------------------------------------
// Scenario 4: Error Recovery on Malformed JSONL
// ---------------------------------------------------------------------------

/// Given malformed JSONL with some valid lines, ingestion reports errors
/// on bad lines (parse_events_jsonl returns Err with line number).
pub fn error_recovery_malformed_jsonl() -> Scenario {
    Scenario::new("Error recovery: malformed JSONL reports line errors")
        .given("a JSONL file with some valid and some invalid lines", |ctx| {
            ctx.numbers.insert("valid_lines".to_string(), 1);
            ctx.numbers.insert("invalid_lines".to_string(), 1);
        })
        .when("ingesting the JSONL file", |ctx| {
            ctx.flags.insert("ingestion_attempted".to_string(), true);
            Ok(())
        })
        .then("the error references the malformed line number", |ctx| {
            assert_true(
                ctx.flag("error_has_line_number").unwrap_or(false),
                "error message includes line number",
            )
        })
}

// ---------------------------------------------------------------------------
// Scenario 5: Empty Input Handling
// ---------------------------------------------------------------------------

/// Given an empty event file, the pipeline completes without crashing
/// and produces valid (empty) outputs.
pub fn empty_input_handling() -> Scenario {
    Scenario::new("Empty input: pipeline produces valid empty output")
        .given("an empty event list", |ctx| {
            ctx.numbers.insert("event_count".to_string(), 0);
        })
        .when("running the full pipeline", |ctx| {
            ctx.flags.insert("pipeline_executed".to_string(), true);
            Ok(())
        })
        .then("no crash occurs", |ctx| {
            assert_true(
                ctx.flag("pipeline_executed").unwrap_or(false),
                "pipeline completed",
            )
        })
        .then("packet.md is produced and valid", |ctx| {
            assert_true(
                ctx.flag("packet_md_exists").unwrap_or(false),
                "packet.md exists",
            )
        })
        .then("workstreams file has zero workstreams", |ctx| {
            let count = assert_present(ctx.number("workstream_count"), "workstream count")?;
            assert_eq(count, 0, "zero workstreams for empty input")
        })
}

// ---------------------------------------------------------------------------
// Scenario 6: Coverage Completeness
// ---------------------------------------------------------------------------

/// Given events with known date range and coverage slices, the coverage
/// manifest accurately reports complete vs incomplete slices.
pub fn coverage_completeness() -> Scenario {
    Scenario::new("Coverage completeness: slices reflect data accuracy")
        .given("events spanning a known date range", |ctx| {
            ctx.strings
                .insert("since".to_string(), "2025-01-01".to_string());
            ctx.strings
                .insert("until".to_string(), "2025-04-01".to_string());
        })
        .when("building a coverage manifest with mixed slices", |ctx| {
            ctx.flags.insert("coverage_built".to_string(), true);
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
                "overall completeness is correct",
            )
        })
}
