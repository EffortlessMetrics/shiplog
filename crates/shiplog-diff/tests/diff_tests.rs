use chrono::{TimeZone, Utc};
use shiplog_diff::*;
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_event(id_str: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts([id_str]),
        kind: EventKind::Manual,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap(),
        actor: Actor {
            login: "user".to_string(),
            id: None,
        },
        repo: RepoRef {
            full_name: "owner/repo".to_string(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: "Test".to_string(),
            description: None,
            started_at: None,
            ended_at: None,
            impact: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

#[test]
fn diff_identical_events() {
    let events = vec![make_event("a"), make_event("b")];
    let diff = diff_events(&events, &events);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
}

#[test]
fn diff_added_events() {
    let old = vec![make_event("a")];
    let new = vec![make_event("a"), make_event("b")];
    let diff = diff_events(&old, &new);
    assert_eq!(diff.added.len(), 1);
    assert!(diff.removed.is_empty());
}

#[test]
fn diff_removed_events() {
    let old = vec![make_event("a"), make_event("b")];
    let new = vec![make_event("a")];
    let diff = diff_events(&old, &new);
    assert!(diff.added.is_empty());
    assert_eq!(diff.removed.len(), 1);
}

#[test]
fn diff_empty_old() {
    let old: Vec<EventEnvelope> = vec![];
    let new = vec![make_event("a")];
    let diff = diff_events(&old, &new);
    assert_eq!(diff.added.len(), 1);
    assert!(diff.removed.is_empty());
}

#[test]
fn diff_empty_new() {
    let old = vec![make_event("a")];
    let new: Vec<EventEnvelope> = vec![];
    let diff = diff_events(&old, &new);
    assert!(diff.added.is_empty());
    assert_eq!(diff.removed.len(), 1);
}

#[test]
fn diff_both_empty() {
    let diff = diff_events(&[], &[]);
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.modified.is_empty());
    assert!(diff.unchanged.is_empty());
}

#[test]
fn event_diff_default() {
    let diff = EventDiff::default();
    assert!(diff.added.is_empty());
    assert!(diff.removed.is_empty());
    assert!(diff.modified.is_empty());
    assert!(diff.unchanged.is_empty());
}
