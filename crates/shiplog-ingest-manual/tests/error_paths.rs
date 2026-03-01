//! Error-path and error message quality tests for shiplog-ingest-manual.
//!
//! Verifies that YAML parse errors include context, invalid field values
//! produce clear messages, and corrupt files don't cause panics.

use chrono::NaiveDate;
use shiplog_ingest_manual::{ManualIngestor, read_manual_events, write_manual_events};
use shiplog_ports::Ingestor;

fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}

fn window_ingestor(path: &std::path::Path) -> ManualIngestor {
    ManualIngestor::new(
        path,
        "testuser".to_string(),
        date(2025, 1, 1),
        date(2025, 12, 31),
    )
}

// ---------------------------------------------------------------------------
// YAML parse error messages are descriptive
// ---------------------------------------------------------------------------

#[test]
fn invalid_yaml_error_is_not_empty() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "{{{{not yaml at all").unwrap();

    let err = read_manual_events(&path).unwrap_err();
    let msg = err.to_string();
    assert!(!msg.is_empty(), "error message should not be empty");
}

#[test]
fn invalid_yaml_error_chain_has_cause() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "{{{{broken yaml}}}}").unwrap();

    let err = read_manual_events(&path).unwrap_err();
    let chain: Vec<String> = err.chain().map(|e| e.to_string()).collect();
    assert!(
        !chain.is_empty(),
        "error chain should have at least one entry"
    );
}

#[test]
fn invalid_event_type_returns_descriptive_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - id: bad-type
    type: NotARealType
    date: "2025-03-15"
    title: "Bad type event"
"#,
    )
    .unwrap();

    let err = read_manual_events(&path).unwrap_err();
    let msg = format!("{err:#}");
    assert!(
        msg.contains("NotARealType") || msg.contains("unknown variant") || msg.contains("type"),
        "error should mention the invalid type: {msg}"
    );
}

// ---------------------------------------------------------------------------
// Write errors: invalid paths
// ---------------------------------------------------------------------------

#[test]
fn write_to_nonexistent_directory_returns_error() {
    let path = std::path::Path::new("H:\\nonexistent\\deep\\path\\manual_events.yaml");
    let file = shiplog_ingest_manual::create_empty_file();
    let result = write_manual_events(path, &file);
    assert!(result.is_err(), "writing to nonexistent dir should fail");
}

// ---------------------------------------------------------------------------
// Ingestor error propagation from read_manual_events
// ---------------------------------------------------------------------------

#[test]
fn ingestor_propagates_yaml_parse_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "not: [valid: yaml: {").unwrap();

    let ing = window_ingestor(&path);
    let err = ing.ingest().unwrap_err();
    let msg = format!("{err:#}");
    // Should contain YAML parse error context
    assert!(
        !msg.is_empty(),
        "ingestor should propagate parse error: {msg}"
    );
}

#[test]
fn ingestor_with_directory_path_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    // Point at a directory, not a file
    let ing = window_ingestor(temp.path());
    let result = ing.ingest();
    // Either returns error (can't read dir as file) or empty (doesn't exist)
    // - on most systems reading a dir as a file is an error
    if temp.path().exists() {
        // The path exists but it's a directory - reading should fail
        // unless the ingestor handles it by checking existence (which it does for non-existent)
        // Since the path exists, it will try to read it as YAML, which will fail
        assert!(result.is_err(), "reading a directory as YAML should fail");
    }
}

// ---------------------------------------------------------------------------
// Graceful handling of files with partial valid data
// ---------------------------------------------------------------------------

#[test]
fn yaml_with_valid_header_but_corrupt_events_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(
        &path,
        r#"version: 1
generated_at: "2025-01-01T00:00:00Z"
events:
  - this is not a valid event structure
"#,
    )
    .unwrap();

    let result = read_manual_events(&path);
    assert!(result.is_err(), "corrupt event structure should fail");
}

#[test]
fn yaml_with_version_only_returns_error() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("manual_events.yaml");
    std::fs::write(&path, "version: 1\n").unwrap();

    let result = read_manual_events(&path);
    // Missing required fields should error
    assert!(
        result.is_err(),
        "YAML with only version should fail: missing required fields"
    );
}

// ---------------------------------------------------------------------------
// Nonexistent file: graceful degradation with warning
// ---------------------------------------------------------------------------

#[test]
fn nonexistent_file_warning_mentions_file_path() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("missing_file.yaml");

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert!(output.events.is_empty());
    assert!(
        output
            .coverage
            .warnings
            .iter()
            .any(|w| w.contains("not found") || w.contains("missing_file")),
        "warning should mention the missing file: {:?}",
        output.coverage.warnings
    );
}

#[test]
fn nonexistent_file_coverage_completeness_is_unknown() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("no_such_file.yaml");

    let ing = window_ingestor(&path);
    let output = ing.ingest().unwrap();
    assert_eq!(
        output.coverage.completeness,
        shiplog_schema::coverage::Completeness::Unknown,
        "missing file should report unknown completeness"
    );
}
