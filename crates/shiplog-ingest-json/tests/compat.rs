//! Backward-compatibility tests for shiplog-ingest-json.
//!
//! Verifies that JSONL from different schema versions still parses,
//! and that extra/missing optional fields are handled gracefully.

use shiplog_ingest_json::parse_events_jsonl;
use shiplog_schema::event::*;

// ── v0.1 JSONL: object-form SourceSystem ────────────────────────────────

#[test]
fn v01_jsonl_object_source_system() {
    let line = r#"{"id":"sha256:old1","kind":"PullRequest","occurred_at":"2025-01-10T08:00:00Z","actor":{"login":"dev","id":1},"repo":{"full_name":"acme/api","html_url":"https://github.com/acme/api","visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":10,"title":"Init","state":"Merged","created_at":"2025-01-09T12:00:00Z","merged_at":"2025-01-10T08:00:00Z","additions":50,"deletions":5,"changed_files":3,"touched_paths_hint":["src/main.rs"],"window":null}},"tags":[],"links":[],"source":{"system":{"Github":null},"url":null,"opaque_id":null}}"#;

    let events = parse_events_jsonl(line, "v0.1").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].source.system, SourceSystem::Github);
    if let EventPayload::PullRequest(pr) = &events[0].payload {
        assert_eq!(pr.number, 10);
    } else {
        panic!("expected PR payload");
    }
}

#[test]
fn v01_jsonl_other_source_system() {
    let line = r#"{"id":"sha256:old2","kind":"Review","occurred_at":"2025-01-15T14:00:00Z","actor":{"login":"rev","id":null},"repo":{"full_name":"acme/api","html_url":null,"visibility":"Private"},"payload":{"type":"Review","data":{"pull_number":10,"pull_title":"Init","submitted_at":"2025-01-15T14:00:00Z","state":"approved","window":null}},"tags":[],"links":[],"source":{"system":{"Other":"gitlab"},"url":null,"opaque_id":null}}"#;

    let events = parse_events_jsonl(line, "v0.1-other").unwrap();
    assert_eq!(
        events[0].source.system,
        SourceSystem::Other("gitlab".into())
    );
}

// ── v0.2 JSONL: string SourceSystem ─────────────────────────────────────

#[test]
fn v02_jsonl_string_source_system() {
    let line = r#"{"id":"sha256:new1","kind":"PullRequest","occurred_at":"2025-03-01T10:00:00Z","actor":{"login":"dev","id":42},"repo":{"full_name":"acme/widgets","html_url":"https://github.com/acme/widgets","visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":7,"title":"Bug fix","state":"Merged","created_at":"2025-02-28T09:00:00Z","merged_at":"2025-03-01T10:00:00Z","additions":3,"deletions":1,"changed_files":1,"touched_paths_hint":[],"window":null}},"tags":["bugfix"],"links":[{"label":"pr","url":"https://github.com/acme/widgets/pull/7"}],"source":{"system":"github","url":null,"opaque_id":"PR_xyz"}}"#;

    let events = parse_events_jsonl(line, "v0.2").unwrap();
    assert_eq!(events[0].source.system, SourceSystem::Github);
    assert_eq!(events[0].tags, vec!["bugfix"]);
}

// ── Mixed versions in one file ──────────────────────────────────────────

#[test]
fn mixed_v01_and_v02_lines() {
    let v01_line = r#"{"id":"sha256:mix1","kind":"PullRequest","occurred_at":"2025-01-10T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":1,"title":"old","state":"Open","created_at":"2025-01-10T00:00:00Z","merged_at":null,"additions":null,"deletions":null,"changed_files":null,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":{"Github":null},"url":null,"opaque_id":null}}"#;
    let v02_line = r#"{"id":"sha256:mix2","kind":"PullRequest","occurred_at":"2025-03-10T00:00:00Z","actor":{"login":"b","id":2},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":2,"title":"new","state":"Merged","created_at":"2025-03-10T00:00:00Z","merged_at":"2025-03-10T00:00:00Z","additions":10,"deletions":0,"changed_files":1,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null}}"#;

    let text = format!("{v01_line}\n{v02_line}");
    let events = parse_events_jsonl(&text, "mixed").unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].source.system, SourceSystem::Github);
    assert_eq!(events[1].source.system, SourceSystem::Github);
}

// ── Extra fields are ignored (forward compat) ───────────────────────────

#[test]
fn extra_top_level_fields_ignored() {
    let line = r#"{"id":"sha256:fwd1","kind":"PullRequest","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":1,"title":"t","state":"Open","created_at":"2025-06-01T00:00:00Z","merged_at":null,"additions":null,"deletions":null,"changed_files":null,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null},"future_field":"ignored","score":99}"#;

    let events = parse_events_jsonl(line, "forward").unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].id.0, "sha256:fwd1");
}

#[test]
fn extra_nested_fields_ignored() {
    // Extra fields in actor, repo, and payload.data
    let line = r#"{"id":"sha256:fwd2","kind":"PullRequest","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null,"avatar_url":"https://img"},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public","stars":100},"payload":{"type":"PullRequest","data":{"number":1,"title":"t","state":"Open","created_at":"2025-06-01T00:00:00Z","merged_at":null,"additions":null,"deletions":null,"changed_files":null,"touched_paths_hint":[],"window":null,"draft":true}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null}}"#;

    let events = parse_events_jsonl(line, "nested-extra").unwrap();
    assert_eq!(events.len(), 1);
}

// ── All three payload types in JSONL ────────────────────────────────────

#[test]
fn all_payload_types_from_jsonl() {
    let pr_line = r#"{"id":"sha256:p1","kind":"PullRequest","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":1,"title":"pr","state":"Merged","created_at":"2025-06-01T00:00:00Z","merged_at":"2025-06-01T00:00:00Z","additions":1,"deletions":0,"changed_files":1,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null}}"#;
    let rev_line = r#"{"id":"sha256:r1","kind":"Review","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"Review","data":{"pull_number":1,"pull_title":"pr","submitted_at":"2025-06-01T00:00:00Z","state":"approved","window":null}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null}}"#;
    let man_line = r#"{"id":"sha256:m1","kind":"Manual","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Unknown"},"payload":{"type":"Manual","data":{"event_type":"Note","title":"manual","description":null,"started_at":null,"ended_at":null,"impact":null}},"tags":[],"links":[],"source":{"system":"manual","url":null,"opaque_id":null}}"#;

    let text = format!("{pr_line}\n{rev_line}\n{man_line}");
    let events = parse_events_jsonl(&text, "all-types").unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].kind, EventKind::PullRequest);
    assert_eq!(events[1].kind, EventKind::Review);
    assert_eq!(events[2].kind, EventKind::Manual);
}

// ── PascalCase SourceSystem in JSONL lines ──────────────────────────────

#[test]
fn pascal_case_source_system_in_jsonl() {
    let line = r#"{"id":"sha256:pc1","kind":"PullRequest","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":1,"title":"t","state":"Open","created_at":"2025-06-01T00:00:00Z","merged_at":null,"additions":null,"deletions":null,"changed_files":null,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":"Github","url":null,"opaque_id":null}}"#;

    let events = parse_events_jsonl(line, "pascal").unwrap();
    assert_eq!(events[0].source.system, SourceSystem::Github);
}

// ── Blank and whitespace lines interspersed ─────────────────────────────

#[test]
fn blank_lines_between_versioned_events() {
    let line = r#"{"id":"sha256:bl1","kind":"PullRequest","occurred_at":"2025-06-01T00:00:00Z","actor":{"login":"a","id":null},"repo":{"full_name":"x/y","html_url":null,"visibility":"Public"},"payload":{"type":"PullRequest","data":{"number":1,"title":"t","state":"Open","created_at":"2025-06-01T00:00:00Z","merged_at":null,"additions":null,"deletions":null,"changed_files":null,"touched_paths_hint":[],"window":null}},"tags":[],"links":[],"source":{"system":"github","url":null,"opaque_id":null}}"#;

    let text = format!("\n\n{line}\n  \n{line}\n\n");
    let events = parse_events_jsonl(&text, "blanks").unwrap();
    assert_eq!(events.len(), 2);
}
