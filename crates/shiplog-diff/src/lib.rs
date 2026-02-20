//! Diff algorithms for comparing shiplog events and packets.
//!
//! Provides functions to compute the difference between two event lists,
//! identifying added, removed, and modified events.

use serde::{Deserialize, Serialize};
use shiplog_ids::EventId;
use shiplog_schema::event::EventEnvelope;
use std::collections::HashSet;

/// Represents the difference between two event collections.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct EventDiff {
    /// Events that are in `new` but not in `old`.
    pub added: Vec<EventEnvelope>,
    /// Events that are in `old` but not in `new`.
    pub removed: Vec<EventEnvelope>,
    /// Events that are in both but have different content.
    pub modified: Vec<EventChange>,
    /// Events that are unchanged.
    pub unchanged: Vec<EventId>,
}

/// Represents a modified event with old and new versions.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EventChange {
    pub event_id: EventId,
    pub old_event: EventEnvelope,
    pub new_event: EventEnvelope,
    pub changes: Vec<FieldChange>,
}

/// Represents a change to a single field.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FieldChange {
    pub field_name: String,
    pub old_value: String,
    pub new_value: String,
}

/// Compute the diff between two event lists.
pub fn diff_events(old_events: &[EventEnvelope], new_events: &[EventEnvelope]) -> EventDiff {
    let old_ids: HashSet<_> = old_events.iter().map(|e| e.id.clone()).collect();
    let new_ids: HashSet<_> = new_events.iter().map(|e| e.id.clone()).collect();

    // Find added events (in new but not in old)
    let added: Vec<_> = new_events
        .iter()
        .filter(|e| !old_ids.contains(&e.id))
        .cloned()
        .collect();

    // Find removed events (in old but not in new)
    let removed: Vec<_> = old_events
        .iter()
        .filter(|e| !new_ids.contains(&e.id))
        .cloned()
        .collect();

    // Find modified and unchanged events
    let mut modified = Vec::new();
    let mut unchanged = Vec::new();

    for old_event in old_events {
        if let Some(new_event) = new_events.iter().find(|e| e.id == old_event.id) {
            let changes = compute_field_changes(old_event, new_event);
            if changes.is_empty() {
                unchanged.push(old_event.id.clone());
            } else {
                modified.push(EventChange {
                    event_id: old_event.id.clone(),
                    old_event: old_event.clone(),
                    new_event: new_event.clone(),
                    changes,
                });
            }
        }
    }

    EventDiff {
        added,
        removed,
        modified,
        unchanged,
    }
}

/// Compute field-level changes between two events.
fn compute_field_changes(old_event: &EventEnvelope, new_event: &EventEnvelope) -> Vec<FieldChange> {
    let mut changes = Vec::new();

    // Compare actor
    if old_event.actor.login != new_event.actor.login {
        changes.push(FieldChange {
            field_name: "actor.login".to_string(),
            old_value: old_event.actor.login.clone(),
            new_value: new_event.actor.login.clone(),
        });
    }

    // Compare tags
    if old_event.tags != new_event.tags {
        changes.push(FieldChange {
            field_name: "tags".to_string(),
            old_value: format!("{:?}", old_event.tags),
            new_value: format!("{:?}", new_event.tags),
        });
    }

    // Compare links
    if old_event.links != new_event.links {
        changes.push(FieldChange {
            field_name: "links".to_string(),
            old_value: format!("{:?}", old_event.links),
            new_value: format!("{:?}", new_event.links),
        });
    }

    // Compare payload
    match (&old_event.payload, &new_event.payload) {
        (
            shiplog_schema::event::EventPayload::Manual(old_m),
            shiplog_schema::event::EventPayload::Manual(new_m),
        ) => {
            if old_m.title != new_m.title {
                changes.push(FieldChange {
                    field_name: "payload.title".to_string(),
                    old_value: old_m.title.clone(),
                    new_value: new_m.title.clone(),
                });
            }
            if old_m.description != new_m.description {
                changes.push(FieldChange {
                    field_name: "payload.description".to_string(),
                    old_value: format!("{:?}", old_m.description),
                    new_value: format!("{:?}", new_m.description),
                });
            }
        }
        _ => {
            // Different payload types - treat as changed
            changes.push(FieldChange {
                field_name: "payload".to_string(),
                old_value: format!("{:?}", old_event.payload),
                new_value: format!("{:?}", new_event.payload),
            });
        }
    }

    changes
}

/// Get a summary of the diff.
pub fn diff_summary(diff: &EventDiff) -> DiffSummary {
    DiffSummary {
        added_count: diff.added.len(),
        removed_count: diff.removed.len(),
        modified_count: diff.modified.len(),
        unchanged_count: diff.unchanged.len(),
    }
}

/// Summary of a diff result.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DiffSummary {
    pub added_count: usize,
    pub removed_count: usize,
    pub modified_count: usize,
    pub unchanged_count: usize,
}

impl DiffSummary {
    /// Total number of events with changes.
    pub fn total_changes(&self) -> usize {
        self.added_count + self.removed_count + self.modified_count
    }
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

    fn make_event(id: &str, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts([id]),
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
                title: title.to_string(),
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
    fn diff_empty_to_events() {
        let old: Vec<EventEnvelope> = vec![];
        let new = vec![make_event("1", "event 1")];
        
        let diff = diff_events(&old, &new);
        
        assert_eq!(diff.added.len(), 1);
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn diff_events_to_empty() {
        let old = vec![make_event("1", "event 1")];
        let new: Vec<EventEnvelope> = vec![];
        
        let diff = diff_events(&old, &new);
        
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed.len(), 1);
        assert!(diff.modified.is_empty());
    }

    #[test]
    fn diff_identifies_added() {
        let old = vec![make_event("a", "event 1")];
        let new = vec![
            make_event("a", "event 1"),
            make_event("b", "event 2"),
        ];
        
        let diff = diff_events(&old, &new);
        
        assert_eq!(diff.added.len(), 1);
        // The added event has id from "b"
    }

    #[test]
    fn diff_identifies_removed() {
        let old = vec![
            make_event("a", "event 1"),
            make_event("b", "event 2"),
        ];
        let new = vec![make_event("a", "event 1")];
        
        let diff = diff_events(&old, &new);
        
        assert_eq!(diff.removed.len(), 1);
        // The removed event has id from "b"
    }

    #[test]
    fn diff_identifies_modified() {
        let mut old_event = make_event("1", "original title");
        let mut new_event = make_event("1", "new title");
        // Make sure IDs match but content differs
        new_event.id = old_event.id.clone();
        
        let diff = diff_events(&[old_event], &[new_event]);
        
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.modified.len(), 1);
    }

    #[test]
    fn diff_identifies_unchanged() {
        let old_event = make_event("1", "same title");
        let new_event = make_event("1", "same title");
        
        let diff = diff_events(&[old_event], &[new_event]);
        
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
        assert_eq!(diff.unchanged.len(), 1);
    }

    #[test]
    fn diff_summary_totals() {
        let old = vec![
            make_event("1", "event 1"),
            make_event("2", "event 2"),
        ];
        let new = vec![
            make_event("1", "event 1 modified"),
            make_event("3", "event 3"),
        ];
        
        let diff = diff_events(&old, &new);
        let summary = diff_summary(&diff);
        
        assert_eq!(summary.added_count, 1);
        assert_eq!(summary.removed_count, 1);
        assert_eq!(summary.modified_count, 1);
        assert_eq!(summary.unchanged_count, 0);
        assert_eq!(summary.total_changes(), 3);
    }
}
