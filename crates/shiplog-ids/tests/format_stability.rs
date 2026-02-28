//! Format stability tests for shiplog-ids.
//!
//! Complements `snapshot.rs` with JSON serialization snapshots and edge-case
//! inputs to guard against accidental format changes.

use shiplog_ids::{EventId, RunId, WorkstreamId};

// ============================================================================
// RunId snapshots (not covered by snapshot.rs)
// ============================================================================

#[test]
fn run_id_display_known_value() {
    let id = RunId("shiplog_1234567890".into());
    insta::assert_snapshot!("run_id_display", id.to_string());
}

#[test]
fn run_id_display_empty_prefix() {
    let id = RunId("_999".into());
    insta::assert_snapshot!("run_id_empty_prefix", id.to_string());
}

// ============================================================================
// JSON serialization of all ID types
// ============================================================================

#[test]
fn event_id_json_serialization() {
    let id = EventId::from_parts(["github", "pr", "octocat/hello-world", "42"]);
    insta::assert_json_snapshot!("event_id_json", id);
}

#[test]
fn workstream_id_json_serialization() {
    let id = WorkstreamId::from_parts(["repo", "octocat/hello-world"]);
    insta::assert_json_snapshot!("workstream_id_json", id);
}

#[test]
fn run_id_json_serialization() {
    let id = RunId("shiplog_1234567890".into());
    insta::assert_json_snapshot!("run_id_json", id);
}

// ============================================================================
// Edge-case parts
// ============================================================================

#[test]
fn event_id_whitespace_parts() {
    let id = EventId::from_parts(["  ", "\t", "\n"]);
    insta::assert_snapshot!("event_id_whitespace", id.to_string());
}

#[test]
fn event_id_long_parts() {
    let long = "a".repeat(1000);
    let id = EventId::from_parts([long.as_str()]);
    insta::assert_snapshot!("event_id_long_part", id.to_string());
}

#[test]
fn event_id_special_chars() {
    let id = EventId::from_parts(["path/with/slashes", "key=value&other=1", "émojis🚀"]);
    insta::assert_snapshot!("event_id_special_chars", id.to_string());
}

#[test]
fn workstream_id_empty_parts() {
    let id = WorkstreamId::from_parts(std::iter::empty::<&str>());
    insta::assert_snapshot!("workstream_id_empty_parts", id.to_string());
}

#[test]
fn workstream_id_unicode() {
    let id = WorkstreamId::from_parts(["日本語", "プロジェクト"]);
    insta::assert_snapshot!("workstream_id_unicode", id.to_string());
}
