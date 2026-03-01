//! Error-path and error message quality tests for shiplog-ingest-json.
//!
//! Verifies that error messages include context (file paths, line numbers),
//! error chains preserve original causes, and partial/corrupt data is
//! handled gracefully.

use shiplog_ingest_json::{JsonIngestor, parse_events_jsonl};
use shiplog_ports::Ingestor;

// ---------------------------------------------------------------------------
// Error message quality: line numbers and context
// ---------------------------------------------------------------------------

#[test]
fn parse_error_on_line_1_mentions_line_1() {
    let err = parse_events_jsonl("{invalid", "test-src").unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("line 1"), "should mention line 1: {msg}");
    assert!(
        msg.contains("test-src"),
        "should mention source name: {msg}"
    );
}

#[test]
fn parse_error_includes_source_name_in_context() {
    let err = parse_events_jsonl("not json", "my-custom-source").unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("my-custom-source"),
        "error should include source identifier: {msg}"
    );
}

#[test]
fn error_chain_preserves_serde_cause() {
    let err = parse_events_jsonl("{\"bad\": true}", "chain-test").unwrap_err();
    let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
    // Should have at least 2 levels: our context + serde error
    assert!(
        chain.len() >= 2,
        "error chain should have context + cause: {chain:?}"
    );
    assert!(
        chain[0].contains("line 1"),
        "top-level should mention line: {:?}",
        chain[0]
    );
}

// ---------------------------------------------------------------------------
// Missing/nonexistent files
// ---------------------------------------------------------------------------

#[test]
fn missing_events_file_error_mentions_read() {
    let tmp = tempfile::tempdir().unwrap();
    let cov_path = tmp.path().join("coverage.manifest.json");
    std::fs::write(&cov_path, make_minimal_coverage_json()).unwrap();

    let ing = JsonIngestor {
        events_path: tmp.path().join("does_not_exist.jsonl"),
        coverage_path: cov_path,
    };
    let err = ing.ingest().unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("read") || msg.contains("does_not_exist"),
        "error should mention file read failure: {msg}"
    );
}

#[test]
fn missing_coverage_file_error_mentions_read() {
    let tmp = tempfile::tempdir().unwrap();
    let events_path = tmp.path().join("ledger.events.jsonl");
    std::fs::write(&events_path, "").unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path: tmp.path().join("does_not_exist.json"),
    };
    let err = ing.ingest().unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("read") || msg.contains("does_not_exist"),
        "error should mention file read failure: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Corrupt coverage JSON
// ---------------------------------------------------------------------------

#[test]
fn corrupt_coverage_json_returns_parse_error() {
    let tmp = tempfile::tempdir().unwrap();
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");

    std::fs::write(&events_path, "").unwrap();
    std::fs::write(&coverage_path, "{{not valid json}}").unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let err = ing.ingest().unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("parse") || msg.contains("coverage"),
        "error should mention parse failure: {msg}"
    );
}

#[test]
fn coverage_with_wrong_structure_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");

    std::fs::write(&events_path, "").unwrap();
    std::fs::write(&coverage_path, r#"{"unexpected": "structure"}"#).unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let err = ing.ingest().unwrap_err();
    assert!(!err.to_string().is_empty(), "error should not be empty");
}

// ---------------------------------------------------------------------------
// Partial data: good lines followed by bad
// ---------------------------------------------------------------------------

#[test]
fn first_bad_line_stops_parsing() {
    // Even if later lines are valid, error on first bad line
    let err = parse_events_jsonl("not json\n{}", "partial").unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("line 1"),
        "should fail on first bad line: {msg}"
    );
}

#[test]
fn empty_input_returns_empty_vec() {
    let result = parse_events_jsonl("", "empty").unwrap();
    assert!(result.is_empty());
}

#[test]
fn only_whitespace_returns_empty_vec() {
    let result = parse_events_jsonl("   \n\t\n  ", "ws").unwrap();
    assert!(result.is_empty());
}

// ---------------------------------------------------------------------------
// Graceful degradation: empty valid files
// ---------------------------------------------------------------------------

#[test]
fn empty_events_with_valid_coverage_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let events_path = tmp.path().join("ledger.events.jsonl");
    let coverage_path = tmp.path().join("coverage.manifest.json");

    std::fs::write(&events_path, "").unwrap();
    std::fs::write(&coverage_path, make_minimal_coverage_json()).unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert!(output.events.is_empty());
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_minimal_coverage_json() -> String {
    r#"{
        "run_id": "test_run",
        "generated_at": "1970-01-01T00:00:00Z",
        "user": "tester",
        "window": { "since": "2025-01-01", "until": "2025-02-01" },
        "mode": "merged",
        "sources": ["test"],
        "slices": [],
        "warnings": [],
        "completeness": "Complete"
    }"#
    .to_string()
}
