//! Filtering logic for shiplog events based on criteria.
//!
//! Provides filter predicates and combinators for filtering events
//! by various criteria such as date range, source system, event kind, tags, and actor.

use chrono::{DateTime, Utc};
use shiplog_schema::event::{EventEnvelope, EventKind, SourceSystem};

/// Filter criteria for events.
#[derive(Clone, Debug, Default)]
pub struct EventFilter {
    /// Filter by source system.
    pub source_systems: Option<Vec<SourceSystem>>,
    /// Filter by event kind.
    pub event_kinds: Option<Vec<EventKind>>,
    /// Filter by minimum date (inclusive).
    pub since: Option<DateTime<Utc>>,
    /// Filter by maximum date (inclusive).
    pub until: Option<DateTime<Utc>>,
    /// Filter by actor login (partial match).
    pub actor_login: Option<String>,
    /// Filter by tags (any match).
    pub tags: Option<Vec<String>>,
    /// Filter by repository full name.
    pub repo_full_name: Option<String>,
}

impl EventFilter {
    /// Create a new empty filter.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source system filter.
    pub fn with_source_system(mut self, source: SourceSystem) -> Self {
        let mut sources = self.source_systems.take().unwrap_or_default();
        sources.push(source);
        self.source_systems = Some(sources);
        self
    }

    /// Add an event kind filter.
    pub fn with_event_kind(mut self, kind: EventKind) -> Self {
        let mut kinds = self.event_kinds.take().unwrap_or_default();
        kinds.push(kind);
        self.event_kinds = Some(kinds);
        self
    }

    /// Set the date range filter.
    pub fn with_date_range(
        mut self,
        since: Option<DateTime<Utc>>,
        until: Option<DateTime<Utc>>,
    ) -> Self {
        self.since = since;
        self.until = until;
        self
    }

    /// Add an actor login filter (partial match).
    pub fn with_actor(mut self, login: impl Into<String>) -> Self {
        self.actor_login = Some(login.into());
        self
    }

    /// Add a tag filter.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        let mut tags = self.tags.take().unwrap_or_default();
        tags.push(tag.into());
        self.tags = Some(tags);
        self
    }

    /// Add a repository filter.
    pub fn with_repo(mut self, full_name: impl Into<String>) -> Self {
        self.repo_full_name = Some(full_name.into());
        self
    }

    /// Check if an event matches this filter.
    pub fn matches(&self, event: &EventEnvelope) -> bool {
        // Check source system
        if let Some(ref sources) = self.source_systems
            && !sources.iter().any(|s| s == &event.source.system)
        {
            return false;
        }

        // Check event kind
        if let Some(ref kinds) = self.event_kinds
            && !kinds.iter().any(|k| k == &event.kind)
        {
            return false;
        }

        // Check date range
        if let Some(since) = self.since
            && event.occurred_at < since
        {
            return false;
        }
        if let Some(until) = self.until
            && event.occurred_at > until
        {
            return false;
        }

        // Check actor login (partial match)
        if let Some(ref login) = self.actor_login
            && !event
                .actor
                .login
                .to_lowercase()
                .contains(&login.to_lowercase())
        {
            return false;
        }

        // Check tags (any match)
        if let Some(ref required_tags) = self.tags
            && !event.tags.iter().any(|t| required_tags.contains(t))
        {
            return false;
        }

        // Check repo
        if let Some(ref repo) = self.repo_full_name
            && event.repo.full_name != *repo
        {
            return false;
        }

        true
    }
}

/// Filter events by the given filter criteria.
pub fn filter_events(events: &[EventEnvelope], filter: &EventFilter) -> Vec<EventEnvelope> {
    events
        .iter()
        .filter(|e| filter.matches(e))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use shiplog_ids::EventId;
    use shiplog_schema::event::{
        Actor, EventPayload, ManualEvent, ManualEventType, RepoRef, RepoVisibility, SourceRef,
    };

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

    fn make_event_at(
        id: &str,
        date: DateTime<Utc>,
        source: SourceSystem,
        kind: EventKind,
    ) -> EventEnvelope {
        let mut e = test_event();
        e.id = EventId::from_parts([id]);
        e.occurred_at = date;
        e.source.system = source;
        e.kind = kind;
        e
    }

    #[test]
    fn filter_by_source_system() {
        let event = test_event();
        let filter = EventFilter::new().with_source_system(SourceSystem::Github);
        assert!(!filter.matches(&event));

        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_by_event_kind() {
        let event = test_event();
        let filter = EventFilter::new().with_event_kind(EventKind::PullRequest);
        assert!(!filter.matches(&event));

        let filter = EventFilter::new().with_event_kind(EventKind::Manual);
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_by_date_range() {
        let event = test_event();
        let filter = EventFilter::new().with_date_range(
            Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            Some(Utc.with_ymd_and_hms(2025, 1, 31, 23, 59, 59).unwrap()),
        );
        assert!(filter.matches(&event));

        let filter = EventFilter::new().with_date_range(
            Some(Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap()),
            None,
        );
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_by_actor() {
        let event = test_event();
        let filter = EventFilter::new().with_actor("test");
        assert!(filter.matches(&event));

        let filter = EventFilter::new().with_actor("other");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_by_tag() {
        let event = test_event();
        let filter = EventFilter::new().with_tag("feature");
        assert!(filter.matches(&event));

        let filter = EventFilter::new().with_tag("bugfix");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_by_repo() {
        let event = test_event();
        let filter = EventFilter::new().with_repo("owner/test");
        assert!(filter.matches(&event));

        let filter = EventFilter::new().with_repo("other/repo");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_combines_criteria() {
        let event = test_event();
        let filter = EventFilter::new()
            .with_source_system(SourceSystem::Manual)
            .with_event_kind(EventKind::Manual)
            .with_actor("test");
        assert!(filter.matches(&event));

        let filter = filter.with_actor("other");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_events_returns_matching() {
        let events = vec![test_event()];
        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        let filtered = filter_events(&events, &filter);
        assert_eq!(filtered.len(), 1);

        let filter = EventFilter::new().with_source_system(SourceSystem::Github);
        let filtered = filter_events(&events, &filter);
        assert!(filtered.is_empty());
    }

    // --- Edge-case tests ---

    #[test]
    fn empty_filter_matches_all() {
        let filter = EventFilter::new();
        assert!(filter.matches(&test_event()));
    }

    #[test]
    fn filter_empty_events_returns_empty() {
        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        let result = filter_events(&[], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_actor_case_insensitive() {
        let event = test_event();
        assert!(EventFilter::new().with_actor("TESTUSER").matches(&event));
        assert!(EventFilter::new().with_actor("TestUser").matches(&event));
        assert!(EventFilter::new().with_actor("testuser").matches(&event));
    }

    #[test]
    fn filter_actor_partial_match() {
        let event = test_event();
        assert!(EventFilter::new().with_actor("test").matches(&event));
        assert!(EventFilter::new().with_actor("user").matches(&event));
        assert!(!EventFilter::new().with_actor("xyz").matches(&event));
    }

    #[test]
    fn filter_multiple_source_systems_any_match() {
        let event = test_event();
        let filter = EventFilter::new()
            .with_source_system(SourceSystem::Github)
            .with_source_system(SourceSystem::Manual);
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_multiple_event_kinds_any_match() {
        let event = test_event();
        let filter = EventFilter::new()
            .with_event_kind(EventKind::PullRequest)
            .with_event_kind(EventKind::Manual);
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_multiple_tags_any_match() {
        let event = test_event();
        let filter = EventFilter::new().with_tag("bugfix").with_tag("feature");
        assert!(filter.matches(&event));

        let filter = EventFilter::new().with_tag("bugfix").with_tag("security");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_date_boundary_since_inclusive() {
        let event = test_event(); // 2025-01-15T10:00:00Z
        let exact = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        assert!(
            EventFilter::new()
                .with_date_range(Some(exact), None)
                .matches(&event)
        );
    }

    #[test]
    fn filter_date_boundary_until_inclusive() {
        let event = test_event(); // 2025-01-15T10:00:00Z
        let exact = Utc.with_ymd_and_hms(2025, 1, 15, 10, 0, 0).unwrap();
        assert!(
            EventFilter::new()
                .with_date_range(None, Some(exact))
                .matches(&event)
        );
    }

    #[test]
    fn filter_since_only() {
        let event = test_event();
        let filter = EventFilter::new().with_date_range(
            Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
            None,
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_until_only() {
        let event = test_event();
        let filter = EventFilter::new().with_date_range(
            None,
            Some(Utc.with_ymd_and_hms(2025, 12, 31, 23, 59, 59).unwrap()),
        );
        assert!(filter.matches(&event));
    }

    #[test]
    fn filter_single_item_list() {
        let events = vec![test_event()];
        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        let result = filter_events(&events, &filter);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, events[0].id);
    }

    #[test]
    fn filter_events_preserves_identity() {
        let events = vec![test_event()];
        let filter = EventFilter::new();
        let result = filter_events(&events, &filter);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], events[0]);
    }

    #[test]
    fn chained_filters_narrow_results() {
        let jan = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
        let feb = Utc.with_ymd_and_hms(2025, 2, 10, 0, 0, 0).unwrap();
        let events = vec![
            make_event_at("a", jan, SourceSystem::Manual, EventKind::Manual),
            make_event_at("b", feb, SourceSystem::Github, EventKind::PullRequest),
        ];

        let first_pass = filter_events(
            &events,
            &EventFilter::new().with_source_system(SourceSystem::Manual),
        );
        assert_eq!(first_pass.len(), 1);

        let second_pass = filter_events(
            &first_pass,
            &EventFilter::new().with_date_range(
                Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
                Some(Utc.with_ymd_and_hms(2025, 1, 31, 0, 0, 0).unwrap()),
            ),
        );
        assert_eq!(second_pass.len(), 1);
        assert_eq!(second_pass[0].id, events[0].id);
    }

    #[test]
    fn filter_mixed_sources_and_kinds() {
        let t = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
        let events = vec![
            make_event_at("a", t, SourceSystem::Manual, EventKind::Manual),
            make_event_at("b", t, SourceSystem::Github, EventKind::PullRequest),
            make_event_at("c", t, SourceSystem::Github, EventKind::Review),
        ];

        let filter = EventFilter::new()
            .with_source_system(SourceSystem::Github)
            .with_event_kind(EventKind::PullRequest);
        let result = filter_events(&events, &filter);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_no_events_match() {
        let t = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
        let events = vec![
            make_event_at("a", t, SourceSystem::Manual, EventKind::Manual),
            make_event_at("b", t, SourceSystem::Manual, EventKind::Manual),
        ];
        let filter = EventFilter::new().with_source_system(SourceSystem::Github);
        assert!(filter_events(&events, &filter).is_empty());
    }

    #[test]
    fn filter_all_events_match() {
        let t = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
        let events = vec![
            make_event_at("a", t, SourceSystem::Manual, EventKind::Manual),
            make_event_at("b", t, SourceSystem::Manual, EventKind::Manual),
        ];
        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        assert_eq!(filter_events(&events, &filter).len(), 2);
    }

    #[test]
    fn filter_event_with_no_tags() {
        let mut event = test_event();
        event.tags.clear();
        let filter = EventFilter::new().with_tag("anything");
        assert!(!filter.matches(&event));
    }

    #[test]
    fn filter_repo_exact_match_only() {
        let event = test_event(); // owner/test
        assert!(!EventFilter::new().with_repo("owner").matches(&event));
        assert!(
            !EventFilter::new()
                .with_repo("owner/test/extra")
                .matches(&event)
        );
        assert!(EventFilter::new().with_repo("owner/test").matches(&event));
    }

    // --- Snapshot tests ---

    #[test]
    fn snapshot_filter_debug_representation() {
        let filter = EventFilter::new()
            .with_source_system(SourceSystem::Github)
            .with_event_kind(EventKind::PullRequest)
            .with_date_range(
                Some(Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()),
                Some(Utc.with_ymd_and_hms(2025, 6, 30, 0, 0, 0).unwrap()),
            )
            .with_actor("octocat")
            .with_tag("feature")
            .with_repo("owner/repo");
        insta::assert_debug_snapshot!(filter);
    }

    #[test]
    fn snapshot_filtered_event_ids() {
        let t = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
        let events = vec![
            make_event_at("alpha", t, SourceSystem::Manual, EventKind::Manual),
            make_event_at("beta", t, SourceSystem::Github, EventKind::PullRequest),
            make_event_at("gamma", t, SourceSystem::Manual, EventKind::Manual),
        ];
        let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
        let result: Vec<String> = filter_events(&events, &filter)
            .iter()
            .map(|e| e.id.0.clone())
            .collect();
        insta::assert_debug_snapshot!(result);
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_date() -> impl Strategy<Value = DateTime<Utc>> {
            (2020i32..2030, 1u32..13, 1u32..29, 0u32..24, 0u32..60)
                .prop_map(|(y, m, d, h, min)| Utc.with_ymd_and_hms(y, m, d, h, min, 0).unwrap())
        }

        proptest! {
            #[test]
            fn empty_filter_always_matches(date in arb_date()) {
                let mut e = test_event();
                e.occurred_at = date;
                prop_assert!(EventFilter::new().matches(&e));
            }

            #[test]
            fn filter_result_is_subset(
                include_manual in proptest::bool::ANY,
                include_github in proptest::bool::ANY,
            ) {
                let t = Utc.with_ymd_and_hms(2025, 1, 10, 0, 0, 0).unwrap();
                let events = vec![
                    make_event_at("a", t, SourceSystem::Manual, EventKind::Manual),
                    make_event_at("b", t, SourceSystem::Github, EventKind::PullRequest),
                ];
                let mut filter = EventFilter::new();
                if include_manual {
                    filter = filter.with_source_system(SourceSystem::Manual);
                }
                if include_github {
                    filter = filter.with_source_system(SourceSystem::Github);
                }
                let result = filter_events(&events, &filter);
                prop_assert!(result.len() <= events.len());
                for r in &result {
                    prop_assert!(events.iter().any(|e| e.id == r.id));
                }
            }

            #[test]
            fn filter_is_idempotent(date in arb_date()) {
                let mut e = test_event();
                e.occurred_at = date;
                let events = vec![e];
                let filter = EventFilter::new().with_source_system(SourceSystem::Manual);
                let first = filter_events(&events, &filter);
                let second = filter_events(&first, &filter);
                prop_assert_eq!(first.len(), second.len());
            }

            #[test]
            fn date_range_filter_consistency(
                since_day in 1u32..28,
                until_day in 1u32..28,
            ) {
                let event = test_event(); // Jan 15
                let since = Utc.with_ymd_and_hms(2025, 1, since_day, 0, 0, 0).unwrap();
                let until = Utc.with_ymd_and_hms(2025, 1, until_day, 23, 59, 59).unwrap();
                let filter = EventFilter::new().with_date_range(Some(since), Some(until));
                let matches = filter.matches(&event);
                if since_day <= 15 && until_day >= 15 {
                    prop_assert!(matches);
                }
                if since_day > 15 {
                    prop_assert!(!matches);
                }
            }
        }
    }
}
