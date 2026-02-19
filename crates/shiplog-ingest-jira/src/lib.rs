//! Jira API ingestor with cache support.
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
use url::Url;

/// Jira issue status filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
    Open,
    InProgress,
    Done,
    Closed,
    All,
}

impl IssueStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Done => "done",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "open" | "to do" => Ok(Self::Open),
            "in_progress" => Ok(Self::InProgress),
            "done" | "resolved" | "closed" => Ok(Self::Done),
            "all" => Ok(Self::All),
            _ => Err(anyhow!("Invalid issue status: {}", s)),
        }
    }
}

#[derive(Debug)]
pub struct JiraIngestor {
    pub user: String,
    pub since: NaiveDate,
    pub until: NaiveDate,
    pub status: IssueStatus,
    pub throttle_ms: u64,
    pub token: Option<String>,
    /// Jira instance URL (e.g., "jira.atlassian.com" or "company.atlassian.net")
    pub instance: String,
    /// Optional cache for API responses
    pub cache: Option<ApiCache>,
}

impl JiraIngestor {
    pub fn new(user: String, since: NaiveDate, until: NaiveDate) -> Self {
        Self {
            user,
            since,
            until,
            status: IssueStatus::Done,
            throttle_ms: 0,
            token: None,
            instance: "jira.atlassian.com".to_string(),
            cache: None,
        }
    }

    /// Set the Jira API token.
    pub fn with_token(mut self, token: String) -> Result<Self> {
        if token.is_empty() {
            return Err(anyhow!("Jira token cannot be empty"));
        }
        self.token = Some(token);
        Ok(self)
    }

    /// Set the Jira instance URL.
    pub fn with_instance(mut self, instance: String) -> Result<Self> {
        if instance.is_empty() {
            return Err(anyhow!("Jira instance cannot be empty"));
        }

        // Validate and normalize the instance URL
        let normalized = if instance.contains("://") {
            if let Ok(u) = Url::parse(&instance) {
                u.host_str()
                    .ok_or_else(|| anyhow!("Invalid Jira instance URL: {}", instance))?
                    .to_string()
            } else {
                return Err(anyhow!("Invalid Jira instance URL: {}", instance));
            }
        } else {
            instance
        };

        self.instance = normalized;
        Ok(self)
    }

    /// Set the issue status filter.
    pub fn with_status(mut self, status: IssueStatus) -> Self {
        self.status = status;
        self
    }

    /// Enable caching with the given cache directory.
    pub fn with_cache(mut self, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_path = cache_dir.into().join("jira-api-cache.db");
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create Jira cache directory {parent:?}"))?;
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
        let scheme = "https";
        format!("{scheme}://{}", self.instance)
    }

    fn api_base_url(&self) -> String {
        format!("https://{}/rest/api/3", self.instance)
    }

    fn client(&self) -> Result<Client> {
        Client::builder()
            .user_agent(concat!("shiplog/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("build reqwest client")
    }

    fn api_url(&self, path: &str) -> String {
        let base = self.api_base_url();
        format!("{}{}", base.trim_end_matches('/'), path)
    }

    fn throttle(&self) {
        if self.throttle_ms > 0 {
            sleep(Duration::from_millis(self.throttle_ms));
        }
    }

    fn get_json<T: DeserializeOwned>(
        &self,
        client: &Client,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let request_url = build_url_with_params(url, params)?;
        let request_url_for_err = request_url.as_str().to_string();

        let mut req = client.get(request_url).header("Accept", "application/json");

        // Jira uses Basic Auth with API token
        if let Some(t) = &self.token {
            req = req.basic_auth(&self.user, Some(t));
        }

        let resp = req
            .send()
            .with_context(|| format!("GET {request_url_for_err}"))?;
        self.throttle();

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();

            // Handle specific Jira error cases
            if status.as_u16() == 401 {
                return Err(anyhow!("Jira authentication failed: invalid or expired token"));
            } else if status.as_u16() == 403 {
                if body.to_lowercase().contains("rate limit") {
                    return Err(anyhow!("Jira API rate limit exceeded"));
                }
                return Err(anyhow!("Jira API access forbidden: {}", body));
            } else if status.as_u16() == 404 {
                return Err(anyhow!("Jira resource not found: {}", body));
            }

            return Err(anyhow!("Jira API error {status}: {body}"));
        }

        resp.json::<T>()
            .with_context(|| format!("parse json from {request_url_for_err}"))
    }

    /// Search for issues using JQL
    fn search_issues(&self, client: &Client) -> Result<(Vec<JiraIssue>, Vec<CoverageSlice>, bool)> {
        let mut slices = Vec::new();
        let mut partial = false;

        // Build JQL query
        let jql = format!(
            "assignee = '{}' AND created >= '{}' AND created <= '{}'",
            self.user,
            self.since.format("%Y-%m-%d"),
            self.until.format("%Y-%m-%d")
        );

        // Add status filter if not "all"
        let jql = if self.status != IssueStatus::All {
            format!("{} AND status = '{}'", jql, self.status.as_str())
        } else {
            jql
        };

        let url = self.api_url("/search");
        let params = vec![
            ("jql", jql.clone()),
            ("fields", "summary,status,created,updated,issuetype,priority".to_string()),
            ("maxResults", "100".to_string()),
        ];

        let search_response: JiraSearchResponse = self.get_json(client, &url, &params)?;

        let issue_count = search_response.total;
        slices.push(CoverageSlice {
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            query: jql.clone(),
            total_count: issue_count,
            fetched: search_response.issues.len() as u64,
            incomplete_results: Some(issue_count > 100),
            notes: vec!["search:jira".to_string()],
        });

        if issue_count > 100 {
            partial = true;
        }

        Ok((search_response.issues, slices, partial))
    }

    /// Convert Jira issues to shiplog events
    fn issues_to_events(&self, issues: Vec<JiraIssue>) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();
        let html_base = self.html_base_url();

        for issue in issues {
            let issue_url = format!(
                "{}/browse/{}",
                html_base, issue.key
            );

            let event = EventEnvelope {
                id: EventId::from_parts(["jira", "issue", &issue.id]),
                kind: EventKind::Manual,
                occurred_at: issue.fields.created,
                actor: Actor {
                    login: issue.fields.assignee
                            .as_ref()
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| self.user.clone()),
                    id: None, // Jira uses string-based account IDs, not u64
                },
                repo: RepoRef {
                    full_name: format!("jira/{}", self.instance),
                    html_url: Some(html_base.clone()),
                    visibility: RepoVisibility::Private,
                },
                payload: EventPayload::Manual(ManualEvent {
                    event_type: ManualEventType::Other,
                    title: issue.fields.summary.clone(),
                    description: issue.fields.description.clone(),
                    started_at: Some(issue.fields.created.date_naive()),
                    ended_at: issue.fields.resolutiondate
                        .as_ref()
                        .map(|d| d.date_naive()),
                    impact: Some(format!("Issue: {}", issue.key)),
                }),
                tags: vec![],
                links: vec![Link {
                    label: "Jira Issue".to_string(),
                    url: issue_url.clone(),
                }],
                source: SourceRef {
                    system: SourceSystem::Other("jira".to_string()),
                    url: Some(issue_url),
                    opaque_id: Some(issue.id),
                },
            };

            events.push(event);
        }

        Ok(events)
    }
}

impl Ingestor for JiraIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let _token = self.token.as_ref().ok_or_else(|| {
            anyhow!("Jira token is required. Set it using with_token() or JIRA_TOKEN environment variable")
        })?;

        let client = self.client()?;
        let run_id = RunId::now("shiplog");
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let warnings: Vec<String> = Vec::new();
        let mut completeness = Completeness::Complete;

        let mut events: Vec<EventEnvelope> = Vec::new();

        // Search for issues
        let (issues, search_slices, search_partial) = self.search_issues(&client)?;
        slices.extend(search_slices);
        if search_partial {
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
            sources: vec!["jira".to_string()],
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

// Jira API types

#[derive(Debug, Deserialize)]
struct JiraSearchResponse {
    start_at: u64,
    max_results: u64,
    total: u64,
    issues: Vec<JiraIssue>,
}

#[derive(Debug, Deserialize)]
struct JiraIssue {
    id: String,
    key: String,
    #[serde(rename = "self")]
    self_url: String,
    fields: JiraIssueFields,
}

#[derive(Debug, Deserialize)]
struct JiraIssueFields {
    summary: String,
    status: JiraIssueStatus,
    created: DateTime<Utc>,
    updated: DateTime<Utc>,
    resolutiondate: Option<DateTime<Utc>>,
    description: Option<String>,
    issuetype: Option<JiraIssueType>,
    priority: Option<JiraPriority>,
    assignee: Option<JiraUser>,
}

#[derive(Debug, Deserialize)]
struct JiraIssueStatus {
    #[serde(rename = "self")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraIssueType {
    #[serde(rename = "self")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraPriority {
    #[serde(rename = "self")]
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraUser {
    #[serde(rename = "accountId")]
    account_id: String,
    name: String,
    #[serde(rename = "displayName")]
    display_name: String,
}

fn build_url_with_params(base: &str, params: &[(&str, String)]) -> Result<Url> {
    let mut url = Url::parse(base).with_context(|| format!("parse url {base}"))?;
    if !params.is_empty() {
        let mut query = url.query_pairs_mut();
        for (k, v) in params {
            query.append_pair(k, v);
        }
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_cache_creates_missing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let cache_dir = temp.path().join("nested").join("cache");

        let ing = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_cache(&cache_dir)
        .unwrap();

        assert!(ing.cache.is_some());
        assert!(cache_dir.join("jira-api-cache.db").exists());
    }

    #[test]
    fn with_in_memory_cache_works() {
        let ing = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_in_memory_cache()
        .unwrap();

        assert!(ing.cache.is_some());
    }

    #[test]
    fn with_token_validates_non_empty() {
        let result = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_token("".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));
    }

    #[test]
    fn with_instance_validates_format() {
        let result = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_instance("".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        let result = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_instance("http://".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid"));
    }

    #[test]
    fn with_instance_strips_protocol() {
        let ing = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_instance("https://company.atlassian.net".to_string())
        .unwrap();

        assert_eq!(ing.instance, "company.atlassian.net");
    }

    #[test]
    fn issue_status_from_str() {
        assert_eq!(IssueStatus::from_str("open").unwrap(), IssueStatus::Open);
        assert_eq!(IssueStatus::from_str("in_progress").unwrap(), IssueStatus::InProgress);
        assert_eq!(IssueStatus::from_str("done").unwrap(), IssueStatus::Done);
        assert_eq!(IssueStatus::from_str("closed").unwrap(), IssueStatus::Closed);
        assert_eq!(IssueStatus::from_str("all").unwrap(), IssueStatus::All);
        assert!(IssueStatus::from_str("invalid").is_err());
    }

    #[test]
    fn issue_status_as_str() {
        assert_eq!(IssueStatus::Open.as_str(), "open");
        assert_eq!(IssueStatus::InProgress.as_str(), "in_progress");
        assert_eq!(IssueStatus::Done.as_str(), "done");
        assert_eq!(IssueStatus::Closed.as_str(), "closed");
        assert_eq!(IssueStatus::All.as_str(), "all");
    }

    #[test]
    fn html_base_url_constructs_correctly() {
        let mut ing = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        ing.instance = "jira.atlassian.com".to_string();
        assert_eq!(ing.html_base_url(), "https://jira.atlassian.com");

        ing.instance = "company.atlassian.net".to_string();
        assert_eq!(ing.html_base_url(), "https://company.atlassian.net");
    }

    #[test]
    fn api_base_url_constructs_correctly() {
        let mut ing = JiraIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        ing.instance = "jira.atlassian.com".to_string();
        assert_eq!(ing.api_base_url(), "https://jira.atlassian.com/rest/api/3");

        ing.instance = "company.atlassian.net".to_string();
        assert_eq!(ing.api_base_url(), "https://company.atlassian.net/rest/api/3");
    }

    #[test]
    fn build_url_with_params_encodes_query_values() {
        let url = build_url_with_params(
            "https://jira.atlassian.com/rest/api/3/search",
            &[
                ("jql", "assignee=alice".to_string()),
                ("fields", "summary,status".to_string()),
            ],
        )
        .unwrap();

        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                ("jql".to_string(), "assignee=alice".to_string()),
                ("fields".to_string(), "summary,status".to_string()),
            ]
        );
    }
}
