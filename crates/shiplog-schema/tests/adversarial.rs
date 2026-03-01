//! Adversarial and negative-input tests for shiplog-schema.

use chrono::{NaiveDate, Utc};
use serde_json::json;
use shiplog_ids::EventId;
use shiplog_schema::coverage::TimeWindow;
use shiplog_schema::event::*;

/// Build a minimal valid EventEnvelope JSON value.
fn valid_event_json() -> serde_json::Value {
    json!({
        "id": EventId::from_parts(["test", "1"]).0,
        "kind": "PullRequest",
        "occurred_at": "2025-01-15T12:00:00Z",
        "actor": { "login": "alice", "id": null },
        "repo": {
            "full_name": "org/repo",
            "html_url": "https://github.com/org/repo",
            "visibility": "Public"
        },
        "payload": {
            "type": "PullRequest",
            "data": {
                "number": 42,
                "title": "Add feature",
                "state": "Merged",
                "created_at": "2025-01-15T12:00:00Z",
                "merged_at": "2025-01-16T12:00:00Z",
                "additions": 10,
                "deletions": 5,
                "changed_files": 3,
                "touched_paths_hint": ["src/main.rs"],
                "window": null
            }
        },
        "tags": ["test"],
        "links": [{ "label": "pr", "url": "https://github.com/org/repo/pull/42" }],
        "source": {
            "system": "github",
            "url": "https://api.github.com/repos/org/repo/pulls/42",
            "opaque_id": null
        }
    })
}

// ── Deserialization: missing required fields ──

#[test]
fn missing_id_field_is_error() {
    let mut v = valid_event_json();
    v.as_object_mut().unwrap().remove("id");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn missing_kind_field_is_error() {
    let mut v = valid_event_json();
    v.as_object_mut().unwrap().remove("kind");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn missing_payload_field_is_error() {
    let mut v = valid_event_json();
    v.as_object_mut().unwrap().remove("payload");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn missing_actor_field_is_error() {
    let mut v = valid_event_json();
    v.as_object_mut().unwrap().remove("actor");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

// ── Deserialization: wrong types ──

#[test]
fn id_as_number_is_error() {
    let mut v = valid_event_json();
    v["id"] = json!(12345);
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn kind_as_number_is_error() {
    let mut v = valid_event_json();
    v["kind"] = json!(99);
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn occurred_at_bad_format_is_error() {
    let mut v = valid_event_json();
    v["occurred_at"] = json!("not-a-date");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn payload_wrong_type_tag_is_error() {
    let mut v = valid_event_json();
    v["payload"]["type"] = json!("Nonexistent");
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

// ── Deserialization: extra / unknown fields ──

#[test]
fn extra_top_level_field_accepted() {
    let mut v = valid_event_json();
    v["surprise_field"] = json!("hi");
    let result = serde_json::from_value::<EventEnvelope>(v);
    assert!(result.is_ok(), "extra fields should be silently ignored");
}

#[test]
fn extra_nested_field_accepted() {
    let mut v = valid_event_json();
    v["actor"]["extra"] = json!("ignored");
    let result = serde_json::from_value::<EventEnvelope>(v);
    assert!(result.is_ok());
}

// ── Deserialization: edge-case strings ──

#[test]
fn empty_string_login() {
    let mut v = valid_event_json();
    v["actor"]["login"] = json!("");
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    assert_eq!(ev.actor.login, "");
}

#[test]
fn unicode_title() {
    let mut v = valid_event_json();
    v["payload"]["data"]["title"] = json!("\u{1F680} \u{65E5}\u{672C}\u{8A9E} \u{0000}test");
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    if let EventPayload::PullRequest(pr) = &ev.payload {
        assert!(pr.title.contains("\u{1F680}"));
    }
}

#[test]
fn very_long_title() {
    let mut v = valid_event_json();
    let long = "x".repeat(100_000);
    v["payload"]["data"]["title"] = json!(long);
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    if let EventPayload::PullRequest(pr) = &ev.payload {
        assert_eq!(pr.title.len(), 100_000);
    }
}

// ── SourceSystem edge cases ──

#[test]
fn source_system_unknown_string() {
    let s = SourceSystem::from_str_lossy("totally_new");
    assert_eq!(s, SourceSystem::Other("totally_new".into()));
}

#[test]
fn source_system_empty_string() {
    let s = SourceSystem::from_str_lossy("");
    assert_eq!(s, SourceSystem::Other("".into()));
}

#[test]
fn source_system_case_insensitive() {
    assert_eq!(SourceSystem::from_str_lossy("GitHub"), SourceSystem::Github);
    assert_eq!(SourceSystem::from_str_lossy("GITHUB"), SourceSystem::Github);
    assert_eq!(SourceSystem::from_str_lossy("github"), SourceSystem::Github);
}

#[test]
fn source_system_serde_string() {
    let v = json!("github");
    let ss: SourceSystem = serde_json::from_value(v).unwrap();
    assert_eq!(ss, SourceSystem::Github);
}

#[test]
fn source_system_serde_map_form() {
    let v = json!({"other": "jira"});
    let ss: SourceSystem = serde_json::from_value(v).unwrap();
    assert_eq!(ss, SourceSystem::Other("jira".into()));
}

// ── EventKind ──

#[test]
fn unknown_event_kind_is_error() {
    let v = json!("SomethingNew");
    assert!(serde_json::from_value::<EventKind>(v).is_err());
}

// ── Review event ──

#[test]
fn review_event_round_trip() {
    let mut v = valid_event_json();
    v["kind"] = json!("Review");
    v["payload"] = json!({
        "type": "Review",
        "data": {
            "pull_number": 42,
            "pull_title": "title",
            "submitted_at": "2025-01-15T12:00:00Z",
            "state": "approved",
            "window": null
        }
    });
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    assert!(matches!(ev.payload, EventPayload::Review(_)));
}

// ── Manual event ──

#[test]
fn manual_event_round_trip() {
    let mut v = valid_event_json();
    v["kind"] = json!("Manual");
    v["payload"] = json!({
        "type": "Manual",
        "data": {
            "event_type": "Note",
            "title": "manual note",
            "description": null,
            "started_at": null,
            "ended_at": null,
            "impact": null
        }
    });
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    assert!(matches!(ev.payload, EventPayload::Manual(_)));
}

// ── Numeric edge cases ──

#[test]
fn negative_additions_accepted() {
    let mut v = valid_event_json();
    v["payload"]["data"]["additions"] = json!(-1);
    // Additions is Option<u32> so negative should fail
    assert!(serde_json::from_value::<EventEnvelope>(v).is_err());
}

#[test]
fn null_optional_fields_accepted() {
    let mut v = valid_event_json();
    v["payload"]["data"]["additions"] = json!(null);
    v["payload"]["data"]["deletions"] = json!(null);
    v["payload"]["data"]["changed_files"] = json!(null);
    v["payload"]["data"]["merged_at"] = json!(null);
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    if let EventPayload::PullRequest(pr) = &ev.payload {
        assert!(pr.additions.is_none());
    }
}

// ── TimeWindow ──

#[test]
fn time_window_inverted_dates_allowed() {
    let since = NaiveDate::from_ymd_opt(2025, 12, 31).unwrap();
    let until = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let tw = TimeWindow { since, until };
    assert!(tw.since > tw.until, "TimeWindow does not validate ordering");
}

#[test]
fn time_window_same_dates() {
    let d = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
    let tw = TimeWindow { since: d, until: d };
    assert_eq!(tw.since, tw.until);
}

// ── Serde round-trip ──

#[test]
fn full_event_serde_round_trip() {
    let ev = EventEnvelope {
        id: EventId::from_parts(["rt"]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "user".into(),
            id: Some(123),
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: Some("https://github.com/org/repo".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: "test".into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(10),
            deletions: Some(5),
            changed_files: Some(3),
            touched_paths_hint: vec!["src/main.rs".into()],
            window: None,
        }),
        tags: vec!["tag1".into()],
        links: vec![Link {
            label: "pr".into(),
            url: "https://example.com".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com".into()),
            opaque_id: None,
        },
    };
    let json = serde_json::to_string(&ev).unwrap();
    let back: EventEnvelope = serde_json::from_str(&json).unwrap();
    assert_eq!(ev.id, back.id);
    assert_eq!(ev.actor.login, back.actor.login);
}

// ── Empty collections ──

#[test]
fn empty_tags_and_links() {
    let mut v = valid_event_json();
    v["tags"] = json!([]);
    v["links"] = json!([]);
    let ev: EventEnvelope = serde_json::from_value(v).unwrap();
    assert!(ev.tags.is_empty());
    assert!(ev.links.is_empty());
}

#[test]
fn repo_visibility_variants() {
    for vis in ["Public", "Private", "Unknown"] {
        let v = json!(vis);
        let r: RepoVisibility = serde_json::from_value(v).unwrap();
        let json = serde_json::to_string(&r).unwrap();
        let back: RepoVisibility = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
