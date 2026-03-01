//! BDD-style scenario tests for shiplog-render-json.

use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::{Completeness, CoverageManifest};
use shiplog_schema::event::EventEnvelope;
use shiplog_testkit::bdd::{Scenario, ScenarioContext};
use shiplog_testkit::fixtures;

// ── Scenario: JSON output is valid JSON ─────────────────────────────────

fn given_realistic_events(ctx: &mut ScenarioContext) {
    let events = fixtures::realistic_quarter_events("alice", "acme/platform");
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("events.jsonl");
    write_events_jsonl(&jsonl_path, &events).unwrap();

    ctx.strings.insert(
        "jsonl_text".into(),
        std::fs::read_to_string(&jsonl_path).unwrap(),
    );
    ctx.numbers
        .insert("event_count".into(), events.len() as u64);
    ctx.paths.insert("tmp_dir".into(), dir.keep());
}

fn when_each_line_is_parsed(ctx: &mut ScenarioContext) -> Result<(), String> {
    let text = ctx.string("jsonl_text").ok_or("missing jsonl_text")?;
    for (i, line) in text.lines().enumerate() {
        serde_json::from_str::<EventEnvelope>(line)
            .map_err(|e| format!("line {i} is not valid JSON: {e}"))?;
    }
    Ok(())
}

fn then_all_lines_are_valid_json(ctx: &ScenarioContext) -> Result<(), String> {
    let text = ctx.string("jsonl_text").ok_or("missing jsonl_text")?;
    let count = ctx.number("event_count").ok_or("missing event_count")?;
    let line_count = text.lines().count() as u64;
    if line_count != count {
        return Err(format!("expected {count} lines, got {line_count}"));
    }
    Ok(())
}

#[test]
fn bdd_json_output_is_valid_json() {
    Scenario::new("JSON output is valid JSON")
        .given(
            "a realistic set of PR events written to JSONL",
            given_realistic_events,
        )
        .when("each line is parsed as JSON", when_each_line_is_parsed)
        .then(
            "all lines are valid EventEnvelopes",
            then_all_lines_are_valid_json,
        )
        .run()
        .expect("JSONL output should contain valid JSON on every line");
}

// ── Scenario: All events appear in output ───────────────────────────────

fn given_numbered_events(ctx: &mut ScenarioContext) {
    let events: Vec<_> = (1..=10)
        .map(|i| shiplog_testkit::pr_event("acme/core", i, &format!("PR number {i}")))
        .collect();
    let dir = tempfile::tempdir().unwrap();
    let jsonl_path = dir.path().join("events.jsonl");
    write_events_jsonl(&jsonl_path, &events).unwrap();

    let ids: Vec<String> = events.iter().map(|e| e.id.to_string()).collect();
    ctx.strings.insert(
        "jsonl_text".into(),
        std::fs::read_to_string(&jsonl_path).unwrap(),
    );
    ctx.strings.insert("expected_ids".into(), ids.join(","));
    ctx.numbers
        .insert("event_count".into(), events.len() as u64);
    ctx.paths.insert("tmp_dir".into(), dir.keep());
}

fn when_output_is_read(ctx: &mut ScenarioContext) -> Result<(), String> {
    let text = ctx.string("jsonl_text").ok_or("missing jsonl_text")?;
    let mut found_ids = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let ev: EventEnvelope =
            serde_json::from_str(line).map_err(|e| format!("line {i} parse error: {e}"))?;
        found_ids.push(ev.id.to_string());
    }
    ctx.strings.insert("found_ids".into(), found_ids.join(","));
    Ok(())
}

fn then_all_event_ids_are_present(ctx: &ScenarioContext) -> Result<(), String> {
    let expected = ctx.string("expected_ids").ok_or("missing expected_ids")?;
    let found = ctx.string("found_ids").ok_or("missing found_ids")?;
    if expected != found {
        return Err(format!(
            "event ID mismatch\n  expected: {expected}\n  found:    {found}"
        ));
    }
    Ok(())
}

fn then_line_count_matches(ctx: &ScenarioContext) -> Result<(), String> {
    let text = ctx.string("jsonl_text").ok_or("missing jsonl_text")?;
    let expected = ctx.number("event_count").ok_or("missing event_count")?;
    let actual = text.lines().count() as u64;
    if actual != expected {
        return Err(format!("expected {expected} lines, got {actual}"));
    }
    Ok(())
}

#[test]
fn bdd_all_events_appear_in_output() {
    Scenario::new("All events appear in output")
        .given("10 numbered PR events", given_numbered_events)
        .when("the JSONL output is read back", when_output_is_read)
        .then(
            "every event ID is present in order",
            then_all_event_ids_are_present,
        )
        .then(
            "the line count matches the event count",
            then_line_count_matches,
        )
        .run()
        .expect("all events should appear in the JSONL output");
}

// ── Scenario: Manifest fields are populated ─────────────────────────────

fn given_coverage_manifest(ctx: &mut ScenarioContext) {
    let cov = fixtures::test_coverage("bob", Completeness::Complete);
    let dir = tempfile::tempdir().unwrap();
    let manifest_path = dir.path().join("coverage.manifest.json");
    write_coverage_manifest(&manifest_path, &cov).unwrap();

    ctx.strings.insert(
        "manifest_text".into(),
        std::fs::read_to_string(&manifest_path).unwrap(),
    );
    ctx.strings.insert("expected_user".into(), "bob".into());
    ctx.strings.insert("expected_mode".into(), "merged".into());
    ctx.paths.insert("tmp_dir".into(), dir.keep());
}

fn when_manifest_is_parsed(ctx: &mut ScenarioContext) -> Result<(), String> {
    let text = ctx
        .string("manifest_text")
        .ok_or("missing manifest_text")?
        .to_owned();
    let manifest: CoverageManifest =
        serde_json::from_str(&text).map_err(|e| format!("manifest parse error: {e}"))?;
    ctx.strings
        .insert("parsed_user".into(), manifest.user.clone());
    ctx.strings
        .insert("parsed_mode".into(), manifest.mode.clone());
    ctx.numbers
        .insert("source_count".into(), manifest.sources.len() as u64);
    ctx.flags.insert(
        "is_complete".into(),
        manifest.completeness == Completeness::Complete,
    );
    Ok(())
}

fn then_user_field_matches(ctx: &ScenarioContext) -> Result<(), String> {
    let expected = ctx.string("expected_user").ok_or("missing expected_user")?;
    let actual = ctx.string("parsed_user").ok_or("missing parsed_user")?;
    if expected != actual {
        return Err(format!(
            "user mismatch: expected '{expected}', got '{actual}'"
        ));
    }
    Ok(())
}

fn then_mode_field_matches(ctx: &ScenarioContext) -> Result<(), String> {
    let expected = ctx.string("expected_mode").ok_or("missing expected_mode")?;
    let actual = ctx.string("parsed_mode").ok_or("missing parsed_mode")?;
    if expected != actual {
        return Err(format!(
            "mode mismatch: expected '{expected}', got '{actual}'"
        ));
    }
    Ok(())
}

fn then_sources_are_present(ctx: &ScenarioContext) -> Result<(), String> {
    let count = ctx.number("source_count").ok_or("missing source_count")?;
    if count == 0 {
        return Err("sources list is empty".into());
    }
    Ok(())
}

fn then_completeness_is_set(ctx: &ScenarioContext) -> Result<(), String> {
    let is_complete = ctx.flag("is_complete").ok_or("missing is_complete flag")?;
    if !is_complete {
        return Err("expected completeness to be Complete".into());
    }
    Ok(())
}

#[test]
fn bdd_manifest_fields_are_populated() {
    Scenario::new("Manifest fields are populated")
        .given(
            "a complete coverage manifest for user bob",
            given_coverage_manifest,
        )
        .when("the manifest JSON is parsed", when_manifest_is_parsed)
        .then("the user field matches", then_user_field_matches)
        .then("the mode field matches", then_mode_field_matches)
        .then("at least one source is listed", then_sources_are_present)
        .then("the completeness flag is set", then_completeness_is_set)
        .run()
        .expect("all manifest fields should be correctly populated");
}

// ── Scenario: JSONL roundtrip preserves event IDs and kinds ─────────────

#[test]
fn bdd_jsonl_roundtrip_preserves_event_fields() {
    Scenario::new("JSONL roundtrip preserves event IDs, kinds, and actor")
        .given("a set of PR events with known fields", |ctx| {
            let events: Vec<_> = (1..=3)
                .map(|i| shiplog_testkit::pr_event("acme/roundtrip", i, &format!("PR {i}")))
                .collect();
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("roundtrip.jsonl");
            write_events_jsonl(&path, &events).unwrap();

            let original_ids: Vec<String> = events.iter().map(|e| e.id.to_string()).collect();
            ctx.strings
                .insert("original_ids".into(), original_ids.join(","));
            ctx.strings
                .insert("jsonl_text".into(), std::fs::read_to_string(&path).unwrap());
            ctx.paths.insert("tmp_dir".into(), dir.keep());
        })
        .when("the JSONL is parsed back into events", |ctx| {
            let text = ctx
                .string("jsonl_text")
                .ok_or("missing jsonl_text")?
                .to_owned();
            let mut parsed_ids = Vec::new();
            let mut all_pr = true;
            let mut all_actor_user = true;
            for line in text.lines() {
                let ev: EventEnvelope =
                    serde_json::from_str(line).map_err(|e| format!("parse error: {e}"))?;
                parsed_ids.push(ev.id.to_string());
                if ev.kind != shiplog_schema::event::EventKind::PullRequest {
                    all_pr = false;
                }
                if ev.actor.login != "user" {
                    all_actor_user = false;
                }
            }
            ctx.strings
                .insert("parsed_ids".into(), parsed_ids.join(","));
            ctx.flags.insert("all_pr_kind".into(), all_pr);
            ctx.flags.insert("all_actor_user".into(), all_actor_user);
            Ok(())
        })
        .then("event IDs match the originals", |ctx| {
            let original = ctx.string("original_ids").ok_or("missing original_ids")?;
            let parsed = ctx.string("parsed_ids").ok_or("missing parsed_ids")?;
            if original != parsed {
                return Err(format!(
                    "ID mismatch\n  original: {original}\n  parsed:   {parsed}"
                ));
            }
            Ok(())
        })
        .then("all events are PullRequest kind", |ctx| {
            let all_pr = ctx.flag("all_pr_kind").ok_or("missing all_pr_kind flag")?;
            if !all_pr {
                return Err("expected all events to be PullRequest kind".into());
            }
            Ok(())
        })
        .then("all events have actor login 'user'", |ctx| {
            let all_user = ctx
                .flag("all_actor_user")
                .ok_or("missing all_actor_user flag")?;
            if !all_user {
                return Err("expected all events to have actor login 'user'".into());
            }
            Ok(())
        })
        .run()
        .expect("JSONL roundtrip should preserve all event fields");
}

// ── Scenario: Coverage manifest JSON is pretty-printed ──────────────────

#[test]
fn bdd_coverage_manifest_is_pretty_printed() {
    Scenario::new("Coverage manifest JSON is pretty-printed and valid")
        .given("a coverage manifest written to disk", |ctx| {
            let cov = fixtures::test_coverage("charlie", Completeness::Partial);
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("coverage.manifest.json");
            write_coverage_manifest(&path, &cov).unwrap();

            ctx.strings.insert(
                "manifest_text".into(),
                std::fs::read_to_string(&path).unwrap(),
            );
            ctx.paths.insert("tmp_dir".into(), dir.keep());
        })
        .when("the manifest text is inspected", |ctx| {
            let text = ctx
                .string("manifest_text")
                .ok_or("missing manifest_text")?
                .to_owned();
            ctx.flags.insert("has_newlines".into(), text.contains('\n'));
            ctx.flags
                .insert("has_indentation".into(), text.contains("  "));
            // Verify it parses as valid JSON
            let parsed: serde_json::Value =
                serde_json::from_str(&text).map_err(|e| format!("invalid JSON: {e}"))?;
            ctx.flags.insert("is_object".into(), parsed.is_object());
            Ok(())
        })
        .then("the JSON should contain newlines", |ctx| {
            let has = ctx
                .flag("has_newlines")
                .ok_or("missing has_newlines flag")?;
            if !has {
                return Err("manifest JSON should be pretty-printed with newlines".into());
            }
            Ok(())
        })
        .then("the JSON should be a valid object", |ctx| {
            let is_obj = ctx.flag("is_object").ok_or("missing is_object flag")?;
            if !is_obj {
                return Err("manifest JSON should be a top-level object".into());
            }
            Ok(())
        })
        .run()
        .expect("coverage manifest should be pretty-printed valid JSON");
}

// ── Scenario: Empty events produce empty JSONL ──────────────────────────

#[test]
fn bdd_empty_events_produce_empty_jsonl() {
    Scenario::new("Empty event list produces empty JSONL file")
        .given("an empty list of events", |ctx| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("empty.jsonl");
            write_events_jsonl(&path, &[]).unwrap();

            ctx.strings
                .insert("jsonl_text".into(), std::fs::read_to_string(&path).unwrap());
            ctx.paths.insert("tmp_dir".into(), dir.keep());
        })
        .when("the JSONL file is read", |ctx| {
            let text = ctx
                .string("jsonl_text")
                .ok_or("missing jsonl_text")?
                .to_owned();
            let line_count = if text.is_empty() {
                0u64
            } else {
                text.lines().count() as u64
            };
            ctx.numbers.insert("line_count".into(), line_count);
            Ok(())
        })
        .then("the JSONL file should have zero lines", |ctx| {
            let count = ctx.number("line_count").ok_or("missing line_count")?;
            if count != 0 {
                return Err(format!("expected 0 lines, got {count}"));
            }
            Ok(())
        })
        .run()
        .expect("empty events should produce empty JSONL file");
}
