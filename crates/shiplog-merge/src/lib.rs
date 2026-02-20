//! Merging utilities for combining multiple event sources.
//!
//! Provides functions to merge and deduplicate events from multiple sources,
//! handling conflicts and preserving the most recent version of each event.

use shiplog_ids::EventId;
use shiplog_schema::event::EventEnvelope;
use std::collections::HashMap;

/// Strategy for handling duplicate events during merge.
#[derive(Clone, Debug, Default)]
pub enum MergeStrategy {
    /// Keep the first event encountered.
    KeepFirst,
    /// Keep the last event encountered (by occurred_at).
    #[default]
    KeepLast,
    /// Keep the event with more complete data.
    KeepMostComplete,
}

/// Merge multiple event lists into one, deduplicating by event ID.
///
/// The strategy determines how to handle conflicts when the same event
/// appears in multiple sources.
pub fn merge_events(
    sources: Vec<Vec<EventEnvelope>>,
    strategy: &MergeStrategy,
) -> Vec<EventEnvelope> {
    let mut events_by_id: HashMap<EventId, EventEnvelope> = HashMap::new();

    for source in sources {
        for event in source {
            match events_by_id.get(&event.id) {
                Some(existing) => {
                    let should_replace = match strategy {
                        MergeStrategy::KeepFirst => false,
                        MergeStrategy::KeepLast => event.occurred_at > existing.occurred_at,
                        MergeStrategy::KeepMostComplete => {
                            completeness_score(&event) > completeness_score(existing)
                        }
                    };
                    if should_replace {
                        events_by_id.insert(event.id.clone(), event);
                    }
                }
                None => {
                    events_by_id.insert(event.id.clone(), event);
                }
            }
        }
    }

    let mut result: Vec<EventEnvelope> = events_by_id.into_values().collect();
    result.sort_by(|a, b| a.occurred_at.cmp(&b.occurred_at));
    result
}

/// Merge two event lists.
pub fn merge_two(
    left: &[EventEnvelope],
    right: &[EventEnvelope],
    strategy: &MergeStrategy,
) -> Vec<EventEnvelope> {
    merge_events(vec![left.to_vec(), right.to_vec()], strategy)
}

/// Calculate a completeness score for an event (higher = more complete).
fn completeness_score(event: &EventEnvelope) -> u32 {
    let mut score = 0;
    
    // Check payload completeness
    match &event.payload {
        shiplog_schema::event::EventPayload::PullRequest(pr) => {
            score += 10;
            if pr.additions.is_some() { score += 1; }
            if pr.deletions.is_some() { score += 1; }
            if pr.changed_files.is_some() { score += 1; }
            if !pr.touched_paths_hint.is_empty() { score += 1; }
        }
        shiplog_schema::event::EventPayload::Review(r) => {
            score += 8;
            if !r.pull_title.is_empty() { score += 1; }
        }
        shiplog_schema::event::EventPayload::Manual(m) => {
            score += 5;
            if m.description.is_some() { score += 2; }
            if m.impact.is_some() { score += 2; }
        }
    }
    
    // Check source completeness
    if event.source.url.is_some() { score += 1; }
    if event.source.opaque_id.is_some() { score += 1; }
    
    // Check links
    if !event.links.is_empty() { score += 2; }
    
    // Check tags
    if !event.tags.is_empty() { score += 1; }
    
    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::event::{
        Actor, EventKind, EventPayload, ManualEvent, ManualEventType, RepoRef, RepoVisibility,
        SourceRef, SourceSystem,
    };

    fn make_event(id: &str, occurred_at: chrono::DateTime<chrono::Utc>) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts([id]),
            kind: EventKind::Manual,
            occurred_at,
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
    fn merge_empty_sources() {
        let result = merge_events(vec![], &MergeStrategy::default());
        assert!(result.is_empty());
    }

    #[test]
    fn merge_single_source() {
        let events = vec![
            make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            make_event("2", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()),
        ];
        let result = merge_events(vec![events], &MergeStrategy::default());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_deduplicates_by_id() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());
        
        let result = merge_events(
            vec![vec![event1.clone()], vec![event2.clone()]],
            &MergeStrategy::KeepLast,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event2.occurred_at);
    }

    #[test]
    fn merge_keeps_first_strategy() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());
        
        let result = merge_events(
            vec![vec![event1.clone()], vec![event2]],
            &MergeStrategy::KeepFirst,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event1.occurred_at);
    }

    #[test]
    fn merge_keeps_last_strategy() {
        let event1 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap());
        let event2 = make_event("1", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap());
        
        let result = merge_events(
            vec![vec![event1], vec![event2.clone()]],
            &MergeStrategy::KeepLast,
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].occurred_at, event2.occurred_at);
    }

    #[test]
    fn merge_two_helper() {
        let left = vec![
            make_event("1", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
        ];
        let right = vec![
            make_event("2", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()),
        ];
        
        let result = merge_two(&left, &right, &MergeStrategy::default());
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_result_is_sorted() {
        let events = vec![
            make_event("a", Utc.with_ymd_and_hms(2025, 1, 3, 0, 0, 0).unwrap()),
            make_event("b", Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            make_event("c", Utc.with_ymd_and_hms(2025, 1, 2, 0, 0, 0).unwrap()),
        ];
        
        let result = merge_events(vec![events], &MergeStrategy::default());
        
        // Should be sorted by occurred_at (Jan 1, Jan 2, Jan 3)
        assert_eq!(result.len(), 3);
        assert!(result[0].occurred_at <= result[1].occurred_at);
        assert!(result[1].occurred_at <= result[2].occurred_at);
    }
}
