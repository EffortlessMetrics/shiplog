use chrono::{TimeZone, Utc};
use shiplog_filter::{EventFilter, filter_events};
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn test_event() -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test"]),
        kind: EventKind::Manual,
        occurred_at: Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap(),
        actor: Actor {
            login: "testuser".to_string(),
            id: Some(123),
        },
        repo: RepoRef {
            full_name: "owner/test".to_string(),
            html_url: Some("https://github.com/owner/test".to_string()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: "Test event".to_string(),
            description: None,
            started_at: None,
            ended_at: None,
            impact: None,
        }),
        tags: vec!["feature".to_string()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

#[test]
fn empty_filter_matches_all() {
    assert!(EventFilter::new().matches(&test_event()));
}

#[test]
fn filter_by_source() {
    let f = EventFilter::new().with_source_system(SourceSystem::Manual);
    assert!(f.matches(&test_event()));
    let f = EventFilter::new().with_source_system(SourceSystem::Github);
    assert!(!f.matches(&test_event()));
}

#[test]
fn filter_by_kind() {
    let f = EventFilter::new().with_event_kind(EventKind::Manual);
    assert!(f.matches(&test_event()));
    let f = EventFilter::new().with_event_kind(EventKind::PullRequest);
    assert!(!f.matches(&test_event()));
}

#[test]
fn filter_by_actor() {
    assert!(EventFilter::new().with_actor("test").matches(&test_event()));
    assert!(
        !EventFilter::new()
            .with_actor("other")
            .matches(&test_event())
    );
}

#[test]
fn filter_by_tag() {
    assert!(
        EventFilter::new()
            .with_tag("feature")
            .matches(&test_event())
    );
    assert!(!EventFilter::new().with_tag("bugfix").matches(&test_event()));
}

#[test]
fn filter_by_repo() {
    assert!(
        EventFilter::new()
            .with_repo("owner/test")
            .matches(&test_event())
    );
    assert!(
        !EventFilter::new()
            .with_repo("other/repo")
            .matches(&test_event())
    );
}

#[test]
fn filter_by_date_range() {
    let f = EventFilter::new().with_date_range(
        Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
        Some(Utc.with_ymd_and_hms(2025, 1, 31, 23, 59, 59).unwrap()),
    );
    assert!(f.matches(&test_event()));
}

#[test]
fn filter_events_fn() {
    let events = vec![test_event()];
    let f = EventFilter::new().with_source_system(SourceSystem::Manual);
    assert_eq!(filter_events(&events, &f).len(), 1);
    let f = EventFilter::new().with_source_system(SourceSystem::Github);
    assert!(filter_events(&events, &f).is_empty());
}

#[test]
fn filter_combined_criteria() {
    let f = EventFilter::new()
        .with_source_system(SourceSystem::Manual)
        .with_event_kind(EventKind::Manual)
        .with_actor("test")
        .with_tag("feature")
        .with_repo("owner/test");
    assert!(f.matches(&test_event()));
}
