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

    ctx.strings
        .insert("jsonl_text".into(), std::fs::read_to_string(&jsonl_path).unwrap());
    ctx.numbers.insert("event_count".into(), events.len() as u64);
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
        .given("a realistic set of PR events written to JSONL", given_realistic_events)
        .when("each line is parsed as JSON", when_each_line_is_parsed)
        .then("all lines are valid EventEnvelopes", then_all_lines_are_valid_json)
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
    ctx.strings
        .insert("jsonl_text".into(), std::fs::read_to_string(&jsonl_path).unwrap());
    ctx.strings.insert("expected_ids".into(), ids.join(","));
    ctx.numbers.insert("event_count".into(), events.len() as u64);
    ctx.paths.insert("tmp_dir".into(), dir.keep());
}

fn when_output_is_read(ctx: &mut ScenarioContext) -> Result<(), String> {
    let text = ctx.string("jsonl_text").ok_or("missing jsonl_text")?;
    let mut found_ids = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let ev: EventEnvelope = serde_json::from_str(line)
            .map_err(|e| format!("line {i} parse error: {e}"))?;
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
        .then("every event ID is present in order", then_all_event_ids_are_present)
        .then("the line count matches the event count", then_line_count_matches)
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
    let text = ctx.string("manifest_text").ok_or("missing manifest_text")?.to_owned();
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
        return Err(format!("user mismatch: expected '{expected}', got '{actual}'"));
    }
    Ok(())
}

fn then_mode_field_matches(ctx: &ScenarioContext) -> Result<(), String> {
    let expected = ctx.string("expected_mode").ok_or("missing expected_mode")?;
    let actual = ctx.string("parsed_mode").ok_or("missing parsed_mode")?;
    if expected != actual {
        return Err(format!("mode mismatch: expected '{expected}', got '{actual}'"));
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
        .given("a complete coverage manifest for user bob", given_coverage_manifest)
        .when("the manifest JSON is parsed", when_manifest_is_parsed)
        .then("the user field matches", then_user_field_matches)
        .then("the mode field matches", then_mode_field_matches)
        .then("at least one source is listed", then_sources_are_present)
        .then("the completeness flag is set", then_completeness_is_set)
        .run()
        .expect("all manifest fields should be correctly populated");
}
