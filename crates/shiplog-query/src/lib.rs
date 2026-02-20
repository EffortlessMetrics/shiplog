//! Query language and filter syntax for searching shiplog events.
//!
//! Provides a simple query parser that converts query strings into
//! filter criteria for searching events.

use shiplog_filter::{filter_events, EventFilter};
use shiplog_schema::event::{EventEnvelope, EventKind, SourceSystem};
use thiserror::Error;

/// Query parsing errors.
#[derive(Error, Debug)]
pub enum QueryError {
    #[error("Invalid query syntax: {0}")]
    SyntaxError(String),
    #[error("Unknown query operator: {0}")]
    UnknownOperator(String),
    #[error("Invalid value for field: {0}")]
    InvalidValue(String),
}

/// Represents a parsed query clause.
#[derive(Clone, Debug)]
enum QueryClause {
    Source(String),
    Kind(String),
    Actor(String),
    Tag(String),
    Repo(String),
    Since(String),
    Until(String),
}

/// A parsed query that can be executed against events.
#[derive(Clone, Debug)]
pub struct Query {
    clauses: Vec<QueryClause>,
}

impl Query {
    /// Parse a query string into a Query.
    ///
    /// Supported syntax:
    /// - `source:github` - Filter by source system
    /// - `kind:pr` - Filter by event kind (pr, review, manual)
    /// - `actor:username` - Filter by actor login
    /// - `tag:value` - Filter by tag
    /// - `repo:owner/name` - Filter by repository
    /// - `since:2025-01-01` - Filter events after date
    /// - `until:2025-01-31` - Filter events before date
    ///
    /// Multiple clauses can be combined with spaces (AND logic).
    pub fn parse(query: &str) -> Result<Self, QueryError> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(Self { clauses: vec![] });
        }

        let mut clauses = Vec::new();
        for part in query.split_whitespace() {
            if let Some((field, value)) = part.split_once(':') {
                let clause = match field.to_lowercase().as_str() {
                    "source" => QueryClause::Source(value.to_string()),
                    "kind" => QueryClause::Kind(value.to_string()),
                    "actor" => QueryClause::Actor(value.to_string()),
                    "tag" => QueryClause::Tag(value.to_string()),
                    "repo" => QueryClause::Repo(value.to_string()),
                    "since" => QueryClause::Since(value.to_string()),
                    "until" => QueryClause::Until(value.to_string()),
                    _ => {
                        return Err(QueryError::UnknownOperator(format!(
                            "Unknown field: {}",
                            field
                        )))
                    }
                };
                clauses.push(clause);
            } else {
                return Err(QueryError::SyntaxError(format!(
                    "Expected 'field:value' but got '{}'",
                    part
                )));
            }
        }

        Ok(Self { clauses })
    }

    /// Convert the query into an EventFilter.
    pub fn to_filter(&self) -> Result<EventFilter, QueryError> {
        let mut filter = EventFilter::new();

        for clause in &self.clauses {
            match clause {
                QueryClause::Source(value) => {
                    let source = match value.to_lowercase().as_str() {
                        "github" => SourceSystem::Github,
                        "json_import" | "jsonimport" => SourceSystem::JsonImport,
                        "local_git" | "localgit" => SourceSystem::LocalGit,
                        "manual" => SourceSystem::Manual,
                        "unknown" => SourceSystem::Unknown,
                        other => SourceSystem::Other(other.to_string()),
                    };
                    filter = filter.with_source_system(source);
                }
                QueryClause::Kind(value) => {
                    let kind = match value.to_lowercase().as_str() {
                        "pr" | "pullrequest" => EventKind::PullRequest,
                        "review" => EventKind::Review,
                        "manual" => EventKind::Manual,
                        _ => {
                            return Err(QueryError::InvalidValue(format!(
                                "Unknown kind: {}",
                                value
                            )))
                        }
                    };
                    filter = filter.with_event_kind(kind);
                }
                QueryClause::Actor(value) => {
                    filter = filter.with_actor(value);
                }
                QueryClause::Tag(value) => {
                    filter = filter.with_tag(value);
                }
                QueryClause::Repo(value) => {
                    filter = filter.with_repo(value);
                }
                QueryClause::Since(value) => {
                    let date = parse_date(value)?;
                    filter = filter.with_date_range(Some(date), None);
                }
                QueryClause::Until(value) => {
                    let date = parse_date(value)?;
                    filter = filter.with_date_range(None, Some(date));
                }
            }
        }

        Ok(filter)
    }

    /// Execute the query against a list of events.
    pub fn execute(&self, events: &[EventEnvelope]) -> Result<Vec<EventEnvelope>, QueryError> {
        let filter = self.to_filter()?;
        Ok(filter_events(events, &filter))
    }
}

/// Parse a date string in YYYY-MM-DD format.
fn parse_date(s: &str) -> Result<chrono::DateTime<chrono::Utc>, QueryError> {
    use chrono::TimeZone;
    
    let parts: Vec<_> = s.split('-').collect();
    if parts.len() != 3 {
        return Err(QueryError::InvalidValue(format!(
            "Expected date in YYYY-MM-DD format, got {}",
            s
        )));
    }

    let year: i32 = parts[0]
        .parse()
        .map_err(|_| QueryError::InvalidValue(format!("Invalid year: {}", parts[0])))?;
    let month: u32 = parts[1]
        .parse()
        .map_err(|_| QueryError::InvalidValue(format!("Invalid month: {}", parts[1])))?;
    let day: u32 = parts[2]
        .parse()
        .map_err(|_| QueryError::InvalidValue(format!("Invalid day: {}", parts[2])))?;

    chrono::Utc
        .with_ymd_and_hms(year, month, day, 0, 0, 0)
        .single()
        .ok_or_else(|| QueryError::InvalidValue(format!("Invalid date: {}", s)))
}

/// Execute a query string against events.
pub fn query_events(query: &str, events: &[EventEnvelope]) -> Result<Vec<EventEnvelope>, QueryError> {
    let query = Query::parse(query)?;
    query.execute(events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_schema::event::{
        Actor, EventPayload, ManualEvent, ManualEventType, RepoRef, RepoVisibility, SourceRef,
    };

    fn make_event() -> EventEnvelope {
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
    fn parse_empty_query() {
        let query = Query::parse("").unwrap();
        assert!(query.clauses.is_empty());
    }

    #[test]
    fn parse_source_clause() {
        let query = Query::parse("source:github").unwrap();
        assert_eq!(query.clauses.len(), 1);
    }

    #[test]
    fn parse_kind_clause() {
        let query = Query::parse("kind:pr").unwrap();
        assert_eq!(query.clauses.len(), 1);
    }

    #[test]
    fn parse_multiple_clauses() {
        let query = Query::parse("source:github kind:pr actor:testuser").unwrap();
        assert_eq!(query.clauses.len(), 3);
    }

    #[test]
    fn parse_invalid_clause() {
        let result = Query::parse("invalid:value");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_colon() {
        let result = Query::parse("source");
        assert!(result.is_err());
    }

    #[test]
    fn query_source() {
        let events = vec![make_event()];
        let result = query_events("source:manual", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("source:github", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_kind() {
        let events = vec![make_event()];
        let result = query_events("kind:manual", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("kind:pr", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_actor() {
        let events = vec![make_event()];
        let result = query_events("actor:testuser", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("actor:other", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_tag() {
        let events = vec![make_event()];
        let result = query_events("tag:feature", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("tag:bugfix", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_repo() {
        let events = vec![make_event()];
        let result = query_events("repo:owner/test", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("repo:other/repo", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_since() {
        let events = vec![make_event()];
        let result = query_events("since:2025-01-01", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("since:2025-02-01", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_until() {
        let events = vec![make_event()];
        let result = query_events("until:2025-01-31", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("until:2025-01-01", &events).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn query_combined() {
        let events = vec![make_event()];
        let result = query_events("source:manual kind:manual", &events).unwrap();
        assert_eq!(result.len(), 1);

        let result = query_events("source:github kind:manual", &events).unwrap();
        assert!(result.is_empty());
    }
}
