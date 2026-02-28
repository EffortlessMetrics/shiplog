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
        let new = vec![make_event("a", "event 1"), make_event("b", "event 2")];

        let diff = diff_events(&old, &new);

        assert_eq!(diff.added.len(), 1);
        // The added event has id from "b"
    }

    #[test]
    fn diff_identifies_removed() {
        let old = vec![make_event("a", "event 1"), make_event("b", "event 2")];
        let new = vec![make_event("a", "event 1")];

        let diff = diff_events(&old, &new);

        assert_eq!(diff.removed.len(), 1);
        // The removed event has id from "b"
    }

    #[test]
    fn diff_identifies_modified() {
        let old_event = make_event("1", "original title");
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
        let old = vec![make_event("1", "event 1"), make_event("2", "event 2")];
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

    // --- Edge-case tests ---

    #[test]
    fn diff_both_empty() {
        let diff = diff_events(&[], &[]);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
        assert!(diff.unchanged.is_empty());
        assert_eq!(diff_summary(&diff).total_changes(), 0);
    }

    #[test]
    fn diff_identical_lists() {
        let events = vec![
            make_event("a", "title a"),
            make_event("b", "title b"),
            make_event("c", "title c"),
        ];
        let diff = diff_events(&events, &events);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.modified.is_empty());
        assert_eq!(diff.unchanged.len(), 3);
    }

    #[test]
    fn diff_completely_disjoint() {
        let old = vec![make_event("a", "old a"), make_event("b", "old b")];
        let new = vec![make_event("c", "new c"), make_event("d", "new d")];
        let diff = diff_events(&old, &new);
        assert_eq!(diff.added.len(), 2);
        assert_eq!(diff.removed.len(), 2);
        assert!(diff.modified.is_empty());
        assert!(diff.unchanged.is_empty());
    }

    #[test]
    fn diff_tag_change_detected() {
        let mut old_event = make_event("1", "title");
        old_event.tags = vec!["alpha".to_string()];
        let mut new_event = make_event("1", "title");
        new_event.tags = vec!["beta".to_string()];

        let diff = diff_events(&[old_event], &[new_event]);
        assert_eq!(diff.modified.len(), 1);
        let change = &diff.modified[0];
        assert!(change.changes.iter().any(|c| c.field_name == "tags"));
    }

    #[test]
    fn diff_actor_change_detected() {
        let old_event = make_event("1", "title");
        let mut new_event = make_event("1", "title");
        new_event.actor.login = "newuser".to_string();

        let diff = diff_events(&[old_event], &[new_event]);
        assert_eq!(diff.modified.len(), 1);
        assert!(
            diff.modified[0]
                .changes
                .iter()
                .any(|c| c.field_name == "actor.login")
        );
    }

    #[test]
    fn diff_description_change_detected() {
        let old_event = make_event("1", "title");
        let mut new_event = make_event("1", "title");
        if let EventPayload::Manual(ref mut m) = new_event.payload {
            m.description = Some("added description".to_string());
        }
        let diff = diff_events(&[old_event], &[new_event]);
        assert_eq!(diff.modified.len(), 1);
        assert!(
            diff.modified[0]
                .changes
                .iter()
                .any(|c| c.field_name == "payload.description")
        );
    }

    #[test]
    fn diff_multiple_field_changes() {
        let old_event = make_event("1", "old title");
        let mut new_event = make_event("1", "new title");
        new_event.actor.login = "different".to_string();
        new_event.tags = vec!["new-tag".to_string()];

        let diff = diff_events(&[old_event], &[new_event]);
        assert_eq!(diff.modified.len(), 1);
        assert!(diff.modified[0].changes.len() >= 3); // title, actor, tags
    }

    #[test]
    fn diff_single_added() {
        let diff = diff_events(&[], &[make_event("x", "new")]);
        let summary = diff_summary(&diff);
        assert_eq!(summary.added_count, 1);
        assert_eq!(summary.removed_count, 0);
        assert_eq!(summary.modified_count, 0);
        assert_eq!(summary.total_changes(), 1);
    }

    #[test]
    fn diff_single_removed() {
        let diff = diff_events(&[make_event("x", "old")], &[]);
        let summary = diff_summary(&diff);
        assert_eq!(summary.added_count, 0);
        assert_eq!(summary.removed_count, 1);
        assert_eq!(summary.total_changes(), 1);
    }

    #[test]
    fn diff_summary_default_is_zero() {
        let s = DiffSummary::default();
        assert_eq!(s.added_count, 0);
        assert_eq!(s.removed_count, 0);
        assert_eq!(s.modified_count, 0);
        assert_eq!(s.unchanged_count, 0);
        assert_eq!(s.total_changes(), 0);
    }

    #[test]
    fn diff_preserves_event_data_in_added() {
        let event = make_event("new-id", "new title");
        let diff = diff_events(&[], std::slice::from_ref(&event));
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].id, event.id);
    }

    #[test]
    fn diff_preserves_event_data_in_removed() {
        let event = make_event("old-id", "old title");
        let diff = diff_events(std::slice::from_ref(&event), &[]);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.removed[0].id, event.id);
    }

    #[test]
    fn diff_change_contains_both_versions() {
        let old_event = make_event("1", "old");
        let mut new_event = make_event("1", "new");
        new_event.id = old_event.id.clone();
        let diff = diff_events(
            std::slice::from_ref(&old_event),
            std::slice::from_ref(&new_event),
        );
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.modified[0].old_event, old_event);
        assert_eq!(diff.modified[0].new_event, new_event);
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_diff_summary() {
        let old = vec![
            make_event("keep", "unchanged"),
            make_event("mod", "before"),
            make_event("del", "removed"),
        ];
        let new = vec![
            make_event("keep", "unchanged"),
            make_event("mod", "after"),
            make_event("add", "new event"),
        ];
        let diff = diff_events(&old, &new);
        let summary = diff_summary(&diff);
        insta::assert_debug_snapshot!(summary);
    }

    #[test]
    fn snapshot_field_changes() {
        let old_event = make_event("1", "original title");
        let mut new_event = make_event("1", "updated title");
        new_event.tags = vec!["new-tag".to_string()];
        new_event.actor.login = "newactor".to_string();
        let diff = diff_events(&[old_event], &[new_event]);
        let field_names: Vec<&str> = diff.modified[0]
            .changes
            .iter()
            .map(|c| c.field_name.as_str())
            .collect();
        insta::assert_debug_snapshot!(field_names);
    }

    #[test]
    fn snapshot_diff_event_default() {
        let diff = EventDiff::default();
        insta::assert_debug_snapshot!(diff);
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn diff_identical_has_no_changes(n in 0usize..6) {
                let events: Vec<_> = (0..n).map(|i| make_event(&format!("e{i}"), &format!("title {i}"))).collect();
                let diff = diff_events(&events, &events);
                prop_assert!(diff.added.is_empty());
                prop_assert!(diff.removed.is_empty());
                prop_assert!(diff.modified.is_empty());
                prop_assert_eq!(diff.unchanged.len(), n);
            }

            #[test]
            fn diff_counts_are_consistent(
                n_shared in 0usize..4,
                n_old_only in 0usize..4,
                n_new_only in 0usize..4,
            ) {
                let shared: Vec<_> = (0..n_shared).map(|i| make_event(&format!("shared{i}"), "same")).collect();
                let old_only: Vec<_> = (0..n_old_only).map(|i| make_event(&format!("old{i}"), "old")).collect();
                let new_only: Vec<_> = (0..n_new_only).map(|i| make_event(&format!("new{i}"), "new")).collect();

                let mut old = shared.clone();
                old.extend(old_only);
                let mut new = shared;
                new.extend(new_only);

                let diff = diff_events(&old, &new);
                let summary = diff_summary(&diff);
                prop_assert_eq!(summary.removed_count, n_old_only);
                prop_assert_eq!(summary.added_count, n_new_only);
                prop_assert_eq!(summary.unchanged_count, n_shared);
            }

            #[test]
            fn diff_reverse_swaps_added_removed(n_old in 0usize..3, n_new in 0usize..3) {
                let old: Vec<_> = (0..n_old).map(|i| make_event(&format!("a{i}"), "old")).collect();
                let new: Vec<_> = (0..n_new).map(|i| make_event(&format!("b{i}"), "new")).collect();
                let forward = diff_summary(&diff_events(&old, &new));
                let reverse = diff_summary(&diff_events(&new, &old));
                prop_assert_eq!(forward.added_count, reverse.removed_count);
                prop_assert_eq!(forward.removed_count, reverse.added_count);
            }
        }
    }
}
