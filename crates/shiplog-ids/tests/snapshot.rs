//! Snapshot tests for shiplog-ids.
//!
//! These lock down the exact ID output for known inputs so that the format
//! cannot accidentally change across releases.

use shiplog_ids::{EventId, WorkstreamId};

// ============================================================================
// EventId snapshots
// ============================================================================

#[test]
fn event_id_single_part() {
    let id = EventId::from_parts(["abc"]);
    insta::assert_snapshot!("event_id_single_part", id.to_string());
}

#[test]
fn event_id_typical_pr() {
    let id = EventId::from_parts(["github", "pr", "octocat/hello-world", "42"]);
    insta::assert_snapshot!("event_id_typical_pr", id.to_string());
}

#[test]
fn event_id_review() {
    let id = EventId::from_parts(["github", "review", "octocat/hello-world", "42", "approved"]);
    insta::assert_snapshot!("event_id_review", id.to_string());
}

#[test]
fn event_id_empty_parts() {
    let id = EventId::from_parts(std::iter::empty::<&str>());
    insta::assert_snapshot!("event_id_empty_parts", id.to_string());
}

#[test]
fn event_id_unicode_parts() {
    let id = EventId::from_parts(["日本語", "テスト"]);
    insta::assert_snapshot!("event_id_unicode_parts", id.to_string());
}

// ============================================================================
// WorkstreamId snapshots
// ============================================================================

#[test]
fn workstream_id_repo_based() {
    let id = WorkstreamId::from_parts(["repo", "octocat/hello-world"]);
    insta::assert_snapshot!("workstream_id_repo_based", id.to_string());
}

#[test]
fn workstream_id_custom_label() {
    let id = WorkstreamId::from_parts(["custom", "infrastructure-migration"]);
    insta::assert_snapshot!("workstream_id_custom_label", id.to_string());
}

#[test]
fn workstream_id_single_part() {
    let id = WorkstreamId::from_parts(["standalone"]);
    insta::assert_snapshot!("workstream_id_single_part", id.to_string());
}
