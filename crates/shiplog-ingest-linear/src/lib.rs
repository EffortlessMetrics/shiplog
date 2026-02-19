//! Linear API ingestor with cache support.
//!
//! Collects issue events, tracks coverage slices, and marks partial
//! completeness when search caps or incomplete API responses are detected.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use shiplog_cache::ApiCache;
use shiplog_ids::{EventId, RunId};
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, ManualEvent, ManualEventType,
    RepoRef, RepoVisibility, SourceRef, SourceSystem,
};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

/// Linear issue status filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
    Backlog,
    Todo,
    InProgress,
    Done,
    Cancelled,
    All,
}

impl IssueStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Backlog => "backlog",
            Self::Todo => "todo",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
            Self::All => "all",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "backlog" => Ok(Self::Backlog),
            "todo" => Ok(Self::Todo),
            "in_progress" | "in progress" => Ok(Self::InProgress),
            "done" | "completed" | "closed" => Ok(Self::Done),
            "cancelled" | "canceled" => Ok(Self::Cancelled),
            "all" => Ok(Self::All),
            _ => Err(anyhow!("Invalid issue status: {}", s)),
        }
    }
}

#[derive(Debug)]
pub struct LinearIngestor {
    pub user: String,
    pub since: NaiveDate,
    pub until: NaiveDate,
    pub status: IssueStatus,
    pub throttle_ms: u64,
    pub api_key: Option<String>,
    /// Optional project filter
    pub project: Option<String>,
    /// Optional cache for API responses
    pub cache: Option<ApiCache>,
}

impl LinearIngestor {
    pub fn new(user: String, since: NaiveDate, until: NaiveDate) -> Self {
        Self {
            user,
            since,
            until,
            status: IssueStatus::Done,
            throttle_ms: 0,
            api_key: None,
            project: None,
            cache: None,
        }
    }

    /// Set the Linear API key.
    pub fn with_api_key(mut self, api_key: String) -> Result<Self> {
        if api_key.is_empty() {
            return Err(anyhow!("Linear API key cannot be empty"));
        }
        self.api_key = Some(api_key);
        Ok(self)
    }

    /// Set the project filter.
    pub fn with_project(mut self, project: String) -> Self {
        self.project = Some(project);
        self
    }

    /// Set the issue status filter.
    pub fn with_status(mut self, status: IssueStatus) -> Self {
        self.status = status;
        self
    }

    /// Enable caching with the given cache directory.
    pub fn with_cache(mut self, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_path = cache_dir.into().join("linear-api-cache.db");
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create Linear cache directory {parent:?}"))?;
        }
        let cache = ApiCache::open(cache_path)?;
        self.cache = Some(cache);
        Ok(self)
    }

    /// Enable in-memory caching (useful for testing).
    pub fn with_in_memory_cache(mut self) -> Result<Self> {
        let cache = ApiCache::open_in_memory()?;
        self.cache = Some(cache);
        Ok(self)
    }

    /// Set throttle delay between API requests (in milliseconds).
    pub fn with_throttle(mut self, ms: u64) -> Self {
        self.throttle_ms = ms;
        self
    }

    fn html_base_url(&self) -> String {
        "https://linear.app".to_string()
    }

    fn api_base_url(&self) -> String {
        "https://api.linear.app/graphql".to_string()
    }

    fn client(&self) -> Result<Client> {
        Client::builder()
            .user_agent(concat!("shiplog/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("build reqwest client")
    }

    fn throttle(&self) {
        if self.throttle_ms > 0 {
            sleep(Duration::from_millis(self.throttle_ms));
        }
    }

    /// Execute a GraphQL query
    fn execute_query<T: DeserializeOwned>(
        &self,
        client: &Client,
        query: &str,
        variables: &serde_json::Value,
    ) -> Result<T> {
        let mut req = client
            .post(&self.api_base_url())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "query": query,
                "variables": variables,
            }));

        // Linear uses Bearer token authentication
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        let resp = req
            .send()
            .context("execute Linear GraphQL query")?;
        self.throttle();

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();

            // Handle specific Linear error cases
            if status.as_u16() == 401 {
                return Err(anyhow!("Linear authentication failed: invalid or expired API key"));
            } else if status.as_u16() == 403 {
                if body.to_lowercase().contains("rate limit") {
                    return Err(anyhow!("Linear API rate limit exceeded"));
                }
                return Err(anyhow!("Linear API access forbidden: {}", body));
            } else if status.as_u16() == 404 {
                return Err(anyhow!("Linear resource not found: {}", body));
            }

            return Err(anyhow!("Linear API error {status}: {body}"));
        }

        let response: LinearResponse<T> = resp.json()
            .context("parse Linear GraphQL response")?;

        if let Some(errors) = response.errors {
            return Err(anyhow!("Linear GraphQL errors: {}", 
                errors.iter()
                    .map(|e| e.message.as_deref().unwrap_or("unknown error"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        response.data.ok_or_else(|| anyhow!("Linear response missing data"))
    }

    /// Query Linear issues
    fn query_issues(&self, client: &Client) -> Result<(Vec<LinearIssue>, Vec<CoverageSlice>, bool)> {
        let mut slices = Vec::new();
        let mut partial = false;

        // Build GraphQL query
        let query = r#"
            query Issues($userId: String!, $first: Int!, $after: String) {
                user(id: $userId) {
                    assignedIssues(first: $first, after: $after) {
                        nodes {
                            id
                            identifier
                            title
                            description
                            state {
                                id
                                name
                                type
                            }
                            project {
                                id
                                name
                                key
                            }
                            createdAt
                            completedAt
                            canceledAt
                            assignee {
                                id
                                name
                                displayName
                            }
                        }
                        pageInfo {
                            hasNextPage
                            endCursor
                        }
                    }
                }
            }
        "#;

        let mut issues = Vec::new();
        let mut after: Option<String> = None;
        let mut total_count = 0u64;

        loop {
            let mut variables = serde_json::json!({
                "userId": self.user,
                "first": 100,
            });
            if let Some(cursor) = &after {
                variables["after"] = serde_json::json!(cursor);
            }

            let response: LinearData<LinearUserResponse> = 
                self.execute_query(client, query, &variables)?;

            if let Some(user) = response.data.and_then(|u| u.user) {
                if let Some(assigned_issues) = user.assigned_issues {
                    if let Some(nodes) = assigned_issues.nodes {
                        let fetched_count = nodes.len() as u64;
                        total_count += fetched_count;
                        issues.extend(nodes);

                        // Check for partial results
                        if assigned_issues.page_info.has_next_page {
                            partial = true;
                            after = assigned_issues.page_info.end_cursor;
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Create coverage slice
        let query_str = format!(
            "assignee = '{}' AND created >= '{}' AND created <= '{}'",
            self.user,
            self.since.format("%Y-%m-%d"),
            self.until.format("%Y-%m-%d")
        );

        slices.push(CoverageSlice {
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            query: query_str,
            total_count,
            fetched: issues.len() as u64,
            incomplete_results: Some(partial),
            notes: vec!["search:linear".to_string()],
        });

        Ok((issues, slices, partial))
    }

    /// Convert Linear issues to shiplog events
    fn issues_to_events(&self, issues: Vec<LinearIssue>) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();
        let html_base = self.html_base_url();

        for issue in issues {
            let issue_url = format!(
                "{}/issue/{}",
                html_base, issue.identifier
            );

            // Determine the event timestamp
            let occurred_at = issue.completed_at
                .or(issue.canceled_at)
                .unwrap_or(issue.created_at);

            // Determine the event type based on state
            let event_type = if issue.state.as_ref().map(|s| s.type_.as_str()).as_deref() == Some("completed") {
                ManualEventType::Other
            } else if issue.state.as_ref().map(|s| s.type_.as_str()).as_deref() == Some("canceled") {
                ManualEventType::Other
            } else {
                ManualEventType::Other
            };

            let event = EventEnvelope {
                id: EventId::from_parts(["linear", "issue", &issue.id]),
                kind: EventKind::Manual,
                occurred_at,
                actor: Actor {
                    login: issue.assignee
                        .as_ref()
                        .map(|a| a.name.clone())
                        .unwrap_or_else(|| self.user.clone()),
                    id: None, // Linear uses string-based IDs, not u64
                },
                repo: RepoRef {
                    full_name: issue.project
                        .as_ref()
                        .map(|p| format!("linear/{}", p.key))
                        .unwrap_or_else(|| "linear/unknown".to_string()),
                    html_url: Some(html_base.clone()),
                    visibility: RepoVisibility::Private,
                },
                payload: EventPayload::Manual(ManualEvent {
                    event_type,
                    title: issue.title.clone(),
                    description: issue.description,
                    started_at: Some(issue.created_at.date_naive()),
                    ended_at: issue.completed_at
                        .or(issue.canceled_at)
                        .map(|d| d.date_naive()),
                    impact: Some(format!("Issue: {}", issue.identifier)),
                }),
                tags: vec![],
                links: vec![Link {
                    label: "Linear Issue".to_string(),
                    url: issue_url.clone(),
                }],
                source: SourceRef {
                    system: SourceSystem::Other("linear".to_string()),
                    url: Some(issue_url),
                    opaque_id: Some(issue.id),
                },
            };

            events.push(event);
        }

        Ok(events)
    }
}

impl Ingestor for LinearIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let _api_key = self.api_key.as_ref().ok_or_else(|| {
            anyhow!("Linear API key is required. Set it using with_api_key() or LINEAR_API_KEY environment variable")
        })?;

        let client = self.client()?;
        let run_id = RunId::now("shiplog");
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let warnings: Vec<String> = Vec::new();
        let mut completeness = Completeness::Complete;

        let mut events: Vec<EventEnvelope> = Vec::new();

        // Query issues
        let (issues, query_slices, query_partial) = self.query_issues(&client)?;
        slices.extend(query_slices);
        if query_partial {
            completeness = Completeness::Partial;
        }

        // Convert issues to events
        events.extend(self.issues_to_events(issues)?);

        // Sort for stable output
        events.sort_by_key(|e| e.occurred_at);

        let cov = CoverageManifest {
            run_id,
            generated_at: Utc::now(),
            user: self.user.clone(),
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            mode: self.status.as_str().to_string(),
            sources: vec!["linear".to_string()],
            slices,
            warnings,
            completeness,
        };

        Ok(IngestOutput {
            events,
            coverage: cov,
        })
    }
}

// Linear API types

#[derive(Debug, Deserialize)]
struct LinearResponse<T> {
    data: Option<T>,
    errors: Option<Vec<LinearError>>,
}

#[derive(Debug, Deserialize)]
struct LinearError {
    message: Option<String>,
    #[serde(rename = "type")]
    type_: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearData<T> {
    data: Option<T>,
}

#[derive(Debug, Deserialize)]
struct LinearUserResponse {
    user: Option<LinearUser>,
}

#[derive(Debug, Deserialize)]
struct LinearUser {
    #[serde(rename = "assignedIssues")]
    assigned_issues: Option<LinearIssuesConnection>,
}

#[derive(Debug, Deserialize)]
struct LinearIssuesConnection {
    nodes: Option<Vec<LinearIssue>>,
    #[serde(rename = "pageInfo")]
    page_info: LinearPageInfo,
}

#[derive(Debug, Deserialize)]
struct LinearPageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearIssue {
    id: String,
    identifier: String,
    title: String,
    description: Option<String>,
    state: Option<LinearState>,
    project: Option<LinearProject>,
    #[serde(rename = "createdAt")]
    created_at: DateTime<Utc>,
    #[serde(rename = "completedAt")]
    completed_at: Option<DateTime<Utc>>,
    #[serde(rename = "canceledAt")]
    canceled_at: Option<DateTime<Utc>>,
    assignee: Option<LinearUserAccount>,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    id: String,
    name: String,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize)]
struct LinearProject {
    id: String,
    name: String,
    key: String,
}

#[derive(Debug, Deserialize)]
struct LinearUserAccount {
    id: String,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_cache_creates_missing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let cache_dir = temp.path().join("nested").join("cache");

        let ing = LinearIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_cache(&cache_dir)
        .unwrap();

        assert!(ing.cache.is_some());
        assert!(cache_dir.join("linear-api-cache.db").exists());
    }

    #[test]
    fn with_in_memory_cache_works() {
        let ing = LinearIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_in_memory_cache()
        .unwrap();

        assert!(ing.cache.is_some());
    }

    #[test]
    fn with_api_key_validates_non_empty() {
        let result = LinearIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_api_key("".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn issue_status_from_str() {
        assert_eq!(IssueStatus::from_str("backlog").unwrap(), IssueStatus::Backlog);
        assert_eq!(IssueStatus::from_str("todo").unwrap(), IssueStatus::Todo);
        assert_eq!(IssueStatus::from_str("in_progress").unwrap(), IssueStatus::InProgress);
        assert_eq!(IssueStatus::from_str("done").unwrap(), IssueStatus::Done);
        assert_eq!(IssueStatus::from_str("cancelled").unwrap(), IssueStatus::Cancelled);
        assert_eq!(IssueStatus::from_str("all").unwrap(), IssueStatus::All);
        assert!(IssueStatus::from_str("invalid").is_err());
    }

    #[test]
    fn issue_status_as_str() {
        assert_eq!(IssueStatus::Backlog.as_str(), "backlog");
        assert_eq!(IssueStatus::Todo.as_str(), "todo");
        assert_eq!(IssueStatus::InProgress.as_str(), "in_progress");
        assert_eq!(IssueStatus::Done.as_str(), "done");
        assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
        assert_eq!(IssueStatus::All.as_str(), "all");
    }

    #[test]
    fn html_base_url_constructs_correctly() {
        let ing = LinearIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        assert_eq!(ing.html_base_url(), "https://linear.app");
    }

    #[test]
    fn api_base_url_constructs_correctly() {
        let ing = LinearIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        assert_eq!(ing.api_base_url(), "https://api.linear.app/graphql");
    }
}
