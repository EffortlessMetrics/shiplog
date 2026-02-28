use chrono::{TimeZone, Utc};
use shiplog_ids::EventId;
use shiplog_merge::*;
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, ManualEvent, ManualEventType, RepoRef,
    RepoVisibility, SourceRef, SourceSystem,
};

fn make_event(id: &str, occurred_at: chrono::DateTime<Utc>) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts([id]),
        kind: EventKind::Manual,
        occurred_at,
        actor: Actor {
            login: "tester".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: format!("Event {id}"),
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
fn conflict_resolution_default_is_prefer_most_recent() {
    let cr = ConflictResolution::default();
    assert_eq!(cr, ConflictResolution::PreferMostRecent);
}

#[test]
fn merge_strategy_default_is_keep_last() {
    let ms = MergeStrategy::default();
    assert!(matches!(ms, MergeStrategy::KeepLast));
}

#[test]
fn conflict_resolution_to_merge_strategy() {
    let ms: MergeStrategy = ConflictResolution::PreferFirst.into();
    assert!(matches!(ms, MergeStrategy::KeepFirst));
    let ms: MergeStrategy = ConflictResolution::PreferMostRecent.into();
    assert!(matches!(ms, MergeStrategy::KeepLast));
    let ms: MergeStrategy = ConflictResolution::PreferMostComplete.into();
    assert!(matches!(ms, MergeStrategy::KeepMostComplete));
}

#[test]
fn merge_report_fields() {
    let report = MergeReport {
        source_count: 2,
        input_event_count: 10,
        output_event_count: 8,
        conflict_count: 2,
        skipped_events: 0,
        warning_count: 1,
    };
    assert_eq!(report.source_count, 2);
    assert_eq!(report.output_event_count, 8);
    assert_eq!(report.conflict_count, 2);
}

#[test]
fn merge_events_empty_sources_returns_empty() {
    let result = merge_events(vec![], &MergeStrategy::default());
    assert!(result.is_empty());
}

#[test]
fn merge_events_single_source_preserves_all() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let events = vec![make_event("a", t1), make_event("b", t2)];
    let result = merge_events(vec![events], &MergeStrategy::default());
    assert_eq!(result.len(), 2);
}

#[test]
fn merge_events_deduplicates_by_id() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let e1 = make_event("dup", t1);
    let e2 = make_event("dup", t2);
    let result = merge_events(vec![vec![e1], vec![e2.clone()]], &MergeStrategy::KeepLast);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].occurred_at, e2.occurred_at);
}

#[test]
fn merge_events_keep_first_keeps_earliest() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let e1 = make_event("dup", t1);
    let e2 = make_event("dup", t2);
    let result = merge_events(vec![vec![e1.clone()], vec![e2]], &MergeStrategy::KeepFirst);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].occurred_at, e1.occurred_at);
}

#[test]
fn merge_events_output_is_sorted_by_time() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let result = merge_events(
        vec![vec![
            make_event("c", t1),
            make_event("a", t2),
            make_event("b", t3),
        ]],
        &MergeStrategy::default(),
    );
    for w in result.windows(2) {
        assert!(w[0].occurred_at <= w[1].occurred_at);
    }
}

#[test]
fn merge_two_no_overlap() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let left = vec![make_event("a", t1)];
    let right = vec![make_event("b", t2)];
    let result = merge_two(&left, &right, &MergeStrategy::default());
    assert_eq!(result.len(), 2);
}

#[test]
fn merge_two_with_overlap_deduplicates() {
    let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let left = vec![make_event("shared", t)];
    let right = vec![make_event("shared", t)];
    let result = merge_two(&left, &right, &MergeStrategy::KeepFirst);
    assert_eq!(result.len(), 1);
}

#[test]
fn merge_two_empty_lists() {
    let result = merge_two(&[], &[], &MergeStrategy::default());
    assert!(result.is_empty());
}

#[test]
fn merge_two_one_empty() {
    let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let left = vec![make_event("a", t)];
    let result = merge_two(&left, &[], &MergeStrategy::default());
    assert_eq!(result.len(), 1);
}

#[test]
fn merge_events_three_sources_partial_overlap() {
    let t1 = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let t2 = Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap();
    let t3 = Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap();
    let result = merge_events(
        vec![
            vec![make_event("a", t1), make_event("shared", t2)],
            vec![make_event("shared", t2), make_event("b", t3)],
            vec![make_event("c", t1)],
        ],
        &MergeStrategy::KeepFirst,
    );
    assert_eq!(result.len(), 4); // a, shared, b, c
}

#[test]
fn merge_events_all_duplicates_collapses_to_one() {
    let t = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap();
    let e = make_event("same", t);
    let result = merge_events(
        vec![vec![e.clone()], vec![e.clone()], vec![e]],
        &MergeStrategy::KeepFirst,
    );
    assert_eq!(result.len(), 1);
}
