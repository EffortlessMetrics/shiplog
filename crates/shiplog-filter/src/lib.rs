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
}
