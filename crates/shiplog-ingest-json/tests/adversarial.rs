//! Adversarial and negative-input tests for shiplog-ingest-json (parse_events_jsonl).

use serde_json::json;
use shiplog_ids::EventId;
use shiplog_ingest_json::parse_events_jsonl;
use shiplog_schema::event::*;

/// Build a minimal valid JSONL line.
fn minimal_valid_line() -> String {
    let ev = json!({
        "id": EventId::from_parts(["test", "1"]).0,
        "kind": "PullRequest",
        "occurred_at": "2025-01-15T12:00:00Z",
        "actor": { "login": "alice", "id": null },
        "repo": {
            "full_name": "org/repo",
            "html_url": null,
            "visibility": "Public"
        },
        "payload": {
            "type": "PullRequest",
            "data": {
                "number": 1,
                "title": "test",
                "state": "Merged",
                "created_at": "2025-01-15T12:00:00Z",
                "merged_at": null,
                "additions": null,
                "deletions": null,
                "changed_files": null,
                "touched_paths_hint": [],
                "window": null
            }
        },
        "tags": [],
        "links": [],
        "source": { "system": "github", "url": null, "opaque_id": null }
    });
    serde_json::to_string(&ev).unwrap()
}

// ── Empty / blank input ──

#[test]
fn empty_string_returns_empty_vec() {
    let events = parse_events_jsonl("", "test").unwrap();
    assert!(events.is_empty());
}

#[test]
fn only_whitespace_returns_empty() {
    let events = parse_events_jsonl("   \n\n  \n  ", "test").unwrap();
    assert!(events.is_empty());
}

#[test]
fn only_newlines_returns_empty() {
    let events = parse_events_jsonl("\n\n\n", "test").unwrap();
    assert!(events.is_empty());
}

// ── Invalid JSON ──

#[test]
fn garbage_text_is_error() {
    let result = parse_events_jsonl("this is not json", "test");
    assert!(result.is_err());
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("line 1"),
        "error should mention line number: {msg}"
    );
}

#[test]
fn partial_json_is_error() {
    let result = parse_events_jsonl("{\"id\":", "test");
    assert!(result.is_err());
}

#[test]
fn json_array_is_error() {
    let line = minimal_valid_line();
    let input = format!("[{line}]");
    let result = parse_events_jsonl(&input, "test");
    assert!(result.is_err());
}

#[test]
fn json_null_line_is_error() {
    let result = parse_events_jsonl("null", "test");
    assert!(result.is_err());
}

#[test]
fn json_number_line_is_error() {
    let result = parse_events_jsonl("42", "test");
    assert!(result.is_err());
}

#[test]
fn json_string_line_is_error() {
    let result = parse_events_jsonl("\"hello\"", "test");
    assert!(result.is_err());
}

// ── Wrong structure ──

#[test]
fn missing_required_field_is_error() {
    let result = parse_events_jsonl("{\"id\": \"abc\"}", "test");
    assert!(result.is_err());
}

#[test]
fn wrong_type_for_field_is_error() {
    let mut v: serde_json::Value = serde_json::from_str(&minimal_valid_line()).unwrap();
    v["kind"] = json!(123);
    let line = serde_json::to_string(&v).unwrap();
    let result = parse_events_jsonl(&line, "test");
    assert!(result.is_err());
}

// ── Mixed valid and invalid ──

#[test]
fn error_on_second_line_reports_line_2() {
    let input = format!("{}\nnot-json", minimal_valid_line());
    let result = parse_events_jsonl(&input, "test");
    assert!(result.is_err());
    let msg = format!("{:#}", result.unwrap_err());
    assert!(msg.contains("line 2"), "should report line 2: {msg}");
}

#[test]
fn blank_lines_between_valid_lines_ok() {
    let line = minimal_valid_line();
    let input = format!("{line}\n\n  \n{line}");
    let events = parse_events_jsonl(&input, "test").unwrap();
    assert_eq!(events.len(), 2);
}

// ── Unicode and special characters ──

#[test]
fn utf8_bom_prefix_is_error() {
    let line = minimal_valid_line();
    let input = format!("\u{FEFF}{line}");
    let result = parse_events_jsonl(&input, "test");
    assert!(result.is_err(), "BOM prefix should cause parse error");
}

#[test]
fn unicode_in_valid_json_accepted() {
    let mut v: serde_json::Value = serde_json::from_str(&minimal_valid_line()).unwrap();
    v["payload"]["data"]["title"] = json!("\u{1F680} rocket \u{65E5}\u{672C}\u{8A9E}");
    let line = serde_json::to_string(&v).unwrap();
    let events = parse_events_jsonl(&line, "test").unwrap();
    if let EventPayload::PullRequest(pr) = &events[0].payload {
        assert!(pr.title.contains("\u{1F680}"));
    }
}

// ── Stress / edge cases ──

#[test]
fn many_valid_lines() {
    let line = minimal_valid_line();
    let input = std::iter::repeat_n(line.as_str(), 1000)
        .collect::<Vec<_>>()
        .join("\n");
    let events = parse_events_jsonl(&input, "test").unwrap();
    assert_eq!(events.len(), 1000);
}

#[test]
fn very_long_single_line() {
    let mut v: serde_json::Value = serde_json::from_str(&minimal_valid_line()).unwrap();
    v["payload"]["data"]["title"] = json!("x".repeat(500_000));
    let line = serde_json::to_string(&v).unwrap();
    let events = parse_events_jsonl(&line, "test").unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn source_name_appears_in_error() {
    let result = parse_events_jsonl("bad", "my-custom-source");
    let msg = format!("{:#}", result.unwrap_err());
    assert!(
        msg.contains("my-custom-source"),
        "source name in error: {msg}"
    );
}

#[test]
fn trailing_newline_does_not_add_extra_event() {
    let line = minimal_valid_line();
    let input = format!("{line}\n");
    let events = parse_events_jsonl(&input, "test").unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn windows_line_endings() {
    let line = minimal_valid_line();
    let input = format!("{line}\r\n{line}\r\n");
    let events = parse_events_jsonl(&input, "test").unwrap();
    assert_eq!(events.len(), 2);
}
