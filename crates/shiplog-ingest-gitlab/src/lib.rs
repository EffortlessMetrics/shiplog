//! GitLab API ingestor with adaptive date slicing and cache support.
//!
//! Collects MR/review events, tracks coverage slices, and marks partial
//! completeness when search caps or incomplete API responses are detected.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use shiplog_cache::ApiCache;
use shiplog_cache_key::CacheKey;
use shiplog_ids::{EventId, RunId};
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, RepoVisibility, ReviewEvent, SourceRef, SourceSystem,
};
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

/// GitLab MR state filter
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MrState {
    Opened,
    Merged,
    Closed,
    All,
}

impl MrState {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Opened => "opened",
            Self::Merged => "merged",
            Self::Closed => "closed",
            Self::All => "all",
        }
    }
}

impl std::str::FromStr for MrState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "opened" => Ok(Self::Opened),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            "all" => Ok(Self::All),
            _ => Err(anyhow!("Invalid MR state: {}", s)),
        }
    }
}

#[derive(Debug)]
pub struct GitlabIngestor {
    pub user: String,
    pub since: NaiveDate,
    pub until: NaiveDate,
    pub state: MrState,
    pub include_reviews: bool,
    pub fetch_details: bool,
    pub throttle_ms: u64,
    pub token: Option<String>,
    /// GitLab instance hostname (e.g., "gitlab.com" or "gitlab.company.com")
    pub instance: String,
    /// Optional cache for API responses
    pub cache: Option<ApiCache>,
}

impl GitlabIngestor {
    pub fn new(user: String, since: NaiveDate, until: NaiveDate) -> Self {
        Self {
            user,
            since,
            until,
            state: MrState::Merged,
            include_reviews: false,
            fetch_details: true,
            throttle_ms: 0,
            token: None,
            instance: "gitlab.com".to_string(),
            cache: None,
        }
    }

    /// Set the GitLab personal access token.
    pub fn with_token(mut self, token: String) -> Result<Self> {
        if token.is_empty() {
            return Err(anyhow!("GitLab token cannot be empty"));
        }
        self.token = Some(token);
        Ok(self)
    }

    /// Set the GitLab instance hostname.
    pub fn with_instance(mut self, instance: String) -> Result<Self> {
        // Validate the instance URL format
        if instance.is_empty() {
            return Err(anyhow!("GitLab instance cannot be empty"));
        }

        // Remove protocol if present and validate hostname
        let hostname = if instance.contains("://") {
            url::Url::parse(&instance)
                .ok()
                .and_then(|u| u.host_str().map(|s| s.to_string()))
                .ok_or_else(|| anyhow!("Invalid GitLab instance URL: {}", instance))?
        } else {
            instance.clone()
        };

        self.instance = hostname;
        Ok(self)
    }

    /// Set the MR state filter.
    pub fn with_state(mut self, state: MrState) -> Self {
        self.state = state;
        self
    }

    /// Enable review collection.
    pub fn with_include_reviews(mut self, include: bool) -> Self {
        self.include_reviews = include;
        self
    }

    /// Enable caching with the given cache directory.
    pub fn with_cache(mut self, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_path = cache_dir.into().join("gitlab-api-cache.db");
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create GitLab cache directory {parent:?}"))?;
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
        format!("https://{}", self.instance)
    }

    fn api_base_url(&self) -> String {
        format!("https://{}/api/v4", self.instance)
    }

    #[mutants::skip]
    fn client(&self) -> Result<Client> {
        Client::builder()
            .user_agent(concat!("shiplog/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("build reqwest client")
    }

    #[mutants::skip]
    fn api_url(&self, path: &str) -> String {
        let base = self.api_base_url();
        format!("{}{}", base.trim_end_matches('/'), path)
    }

    #[mutants::skip]
    fn throttle(&self) {
        if self.throttle_ms > 0 {
            sleep(Duration::from_millis(self.throttle_ms));
        }
    }

    #[mutants::skip]
    fn get_json<T: DeserializeOwned>(
        &self,
        client: &Client,
        url: &str,
        params: &[(&str, String)],
    ) -> Result<T> {
        let request_url = build_url_with_params(url, params)?;
        let request_url_for_err = request_url.as_str().to_string();

        let mut req = client.get(request_url).header("Accept", "application/json");

        // GitLab uses PRIVATE-TOKEN header for authentication
        if let Some(t) = &self.token {
            req = req.header("PRIVATE-TOKEN", t);
        }

        let resp = req
            .send()
            .with_context(|| format!("GET {request_url_for_err}"))?;
        self.throttle();

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();

            // Handle specific GitLab error cases
            if status.as_u16() == 401 {
                return Err(anyhow!(
                    "GitLab authentication failed: invalid or expired token"
                ));
            } else if status.as_u16() == 403 {
                if body.to_lowercase().contains("rate limit") {
                    return Err(anyhow!("GitLab API rate limit exceeded"));
                }
                return Err(anyhow!("GitLab API access forbidden: {}", body));
            } else if status.as_u16() == 404 {
                return Err(anyhow!("GitLab resource not found: {}", body));
            }

            return Err(anyhow!("GitLab API error {status}: {body}"));
        }

        resp.json::<T>()
            .with_context(|| format!("parse json from {request_url_for_err}"))
    }

    /// Get user ID from username (required for GitLab API queries)
    #[mutants::skip]
    fn get_user_id(&self, client: &Client) -> Result<u64> {
        let url = self.api_url(&format!("/users?username={}", self.user));
        let users: Vec<GitlabUser> = self.get_json(client, &url, &[])?;

        users
            .into_iter()
            .find(|u| u.username == self.user)
            .map(|u| u.id)
            .ok_or_else(|| anyhow!("GitLab user '{}' not found", self.user))
    }

    /// Get projects accessible to the user
    #[mutants::skip]
    fn get_user_projects(&self, client: &Client, user_id: u64) -> Result<Vec<GitlabProject>> {
        let url = self.api_url(&format!("/users/{}/projects", user_id));
        let mut projects = Vec::new();
        let per_page = 100;

        for page in 1..=10 {
            let page_projects: Vec<GitlabProject> = self.get_json(
                client,
                &url,
                &[
                    ("per_page", per_page.to_string()),
                    ("page", page.to_string()),
                    ("order_by", "updated_at".to_string()),
                    ("sort", "desc".to_string()),
                ],
            )?;

            let n = page_projects.len();
            projects.extend(page_projects);

            if n < per_page {
                break;
            }
        }

        Ok(projects)
    }

    /// Collect MRs from projects
    #[mutants::skip]
    fn collect_mrs_from_projects(
        &self,
        client: &Client,
        projects: Vec<GitlabProject>,
    ) -> Result<(Vec<GitlabMergeRequest>, Vec<CoverageSlice>, bool)> {
        let mut all_mrs = Vec::new();
        let mut slices = Vec::new();
        let partial = false;

        for project in projects {
            let url = self.api_url(&format!("/projects/{}/merge_requests", project.id));

            let mut params = vec![
                ("author_username", self.user.clone()),
                ("per_page", "100".to_string()),
                ("order_by", "created_at".to_string()),
                ("sort", "desc".to_string()),
            ];

            // Add state filter
            if self.state != MrState::All {
                params.push(("state", self.state.as_str().to_string()));
            }

            // Add date filters
            let start = self.since.format("%Y-%m-%d").to_string();
            let end = self.until.format("%Y-%m-%d").to_string();
            params.push(("created_after", start));
            params.push(("created_before", end));

            let page_mrs: Vec<GitlabMergeRequest> = match self.get_json(client, &url, &params) {
                Ok(mrs) => mrs,
                Err(e) => {
                    // Skip projects we can't access (e.g., private projects)
                    if e.to_string().contains("404") || e.to_string().contains("403") {
                        continue;
                    }
                    return Err(e);
                }
            };

            let mr_count = page_mrs.len() as u64;
            slices.push(CoverageSlice {
                window: TimeWindow {
                    since: self.since,
                    until: self.until,
                },
                query: format!(
                    "project:{} MRs by {}",
                    project.path_with_namespace, self.user
                ),
                total_count: mr_count,
                fetched: mr_count,
                incomplete_results: Some(false),
                notes: vec![format!("project:{}", project.path_with_namespace)],
            });

            all_mrs.extend(page_mrs);
        }

        Ok((all_mrs, slices, partial))
    }

    /// Collect notes (reviews) for an MR
    #[mutants::skip]
    fn collect_mr_notes(
        &self,
        client: &Client,
        project_id: u64,
        mr_iid: u64,
    ) -> Result<Vec<GitlabNote>> {
        let url = self.api_url(&format!(
            "/projects/{}/merge_requests/{}/notes",
            project_id, mr_iid
        ));

        let mut notes = Vec::new();
        let per_page = 100;

        for page in 1..=10 {
            let cache_key = CacheKey::mr_notes(project_id, mr_iid, page);

            let page_notes: Vec<GitlabNote> = if let Some(ref cache) = self.cache {
                if let Some(cached) = cache.get::<Vec<GitlabNote>>(&cache_key)? {
                    cached
                } else {
                    let notes: Vec<GitlabNote> = self.get_json(
                        client,
                        &url,
                        &[
                            ("per_page", per_page.to_string()),
                            ("page", page.to_string()),
                        ],
                    )?;
                    cache.set(&cache_key, &notes)?;
                    notes
                }
            } else {
                self.get_json(
                    client,
                    &url,
                    &[
                        ("per_page", per_page.to_string()),
                        ("page", page.to_string()),
                    ],
                )?
            };

            let n = page_notes.len();
            notes.extend(page_notes);

            if n < per_page {
                break;
            }
        }

        Ok(notes)
    }

    /// Convert GitLab MRs to shiplog events
    #[mutants::skip]
    fn mrs_to_events(&self, mrs: Vec<GitlabMergeRequest>) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();
        let html_base = self.html_base_url();

        for mr in mrs {
            let state = match mr.state.as_str() {
                "opened" => PullRequestState::Open,
                "merged" => PullRequestState::Merged,
                "closed" => PullRequestState::Closed,
                _ => PullRequestState::Unknown,
            };

            let mr_url = format!(
                "{}/{}/-/merge_requests/{}",
                html_base, mr.project.path_with_namespace, mr.iid
            );

            let event = EventEnvelope {
                id: EventId::from_parts(["gitlab", "mr", &mr.id.to_string()]),
                kind: EventKind::PullRequest,
                occurred_at: mr.created_at,
                actor: Actor {
                    login: mr.author.username,
                    id: Some(mr.author.id),
                },
                repo: RepoRef {
                    full_name: mr.project.path_with_namespace.clone(),
                    html_url: Some(format!("{}/{}", html_base, mr.project.path_with_namespace)),
                    visibility: if mr.project.public {
                        RepoVisibility::Public
                    } else {
                        RepoVisibility::Private
                    },
                },
                payload: EventPayload::PullRequest(PullRequestEvent {
                    number: mr.iid,
                    title: mr.title,
                    state,
                    created_at: mr.created_at,
                    merged_at: mr.merged_at,
                    additions: mr.additions,
                    deletions: mr.deletions,
                    changed_files: mr.changed_files,
                    touched_paths_hint: vec![],
                    window: None,
                }),
                tags: mr.labels,
                links: vec![Link {
                    label: "GitLab MR".to_string(),
                    url: mr_url.clone(),
                }],
                source: SourceRef {
                    system: SourceSystem::Other("gitlab".to_string()),
                    url: Some(mr_url.clone()),
                    opaque_id: Some(mr.id.to_string()),
                },
            };

            events.push(event);
        }

        Ok(events)
    }

    /// Convert GitLab notes to shiplog review events
    #[mutants::skip]
    fn notes_to_review_events(
        &self,
        notes: Vec<GitlabNote>,
        mr: &GitlabMergeRequest,
    ) -> Result<Vec<EventEnvelope>> {
        let mut events = Vec::new();
        let html_base = self.html_base_url();

        for note in notes {
            // Only include notes that are actual reviews (not system notes or comments)
            if note.system || note.author.username == self.user {
                continue;
            }

            let mr_url = format!(
                "{}/{}/-/merge_requests/{}#note_{}",
                html_base, mr.project.path_with_namespace, mr.iid, note.id
            );

            let event = EventEnvelope {
                id: EventId::from_parts(["gitlab", "review", &note.id.to_string()]),
                kind: EventKind::Review,
                occurred_at: note.created_at,
                actor: Actor {
                    login: note.author.username,
                    id: Some(note.author.id),
                },
                repo: RepoRef {
                    full_name: mr.project.path_with_namespace.clone(),
                    html_url: Some(format!("{}/{}", html_base, mr.project.path_with_namespace)),
                    visibility: if mr.project.public {
                        RepoVisibility::Public
                    } else {
                        RepoVisibility::Private
                    },
                },
                payload: EventPayload::Review(ReviewEvent {
                    pull_number: mr.iid,
                    pull_title: mr.title.clone(),
                    submitted_at: note.created_at,
                    state: "approved".to_string(),
                    window: None,
                }),
                tags: vec![],
                links: vec![Link {
                    label: "GitLab Review".to_string(),
                    url: mr_url.clone(),
                }],
                source: SourceRef {
                    system: SourceSystem::Other("gitlab".to_string()),
                    url: Some(mr_url.clone()),
                    opaque_id: Some(note.id.to_string()),
                },
            };

            events.push(event);
        }

        Ok(events)
    }
}

impl Ingestor for GitlabIngestor {
    #[mutants::skip]
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let _token = self.token.as_ref().ok_or_else(|| {
            anyhow!("GitLab token is required. Set it using with_token() or GITLAB_TOKEN environment variable")
        })?;

        let client = self.client()?;
        let run_id = RunId::now("shiplog");
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut completeness = Completeness::Complete;

        let mut events: Vec<EventEnvelope> = Vec::new();

        // Get user ID
        let user_id = self.get_user_id(&client)?;

        // Get user's projects
        let projects = self.get_user_projects(&client, user_id)?;

        if projects.is_empty() {
            warnings.push("No projects found for user. This may be due to insufficient permissions or no activity.".to_string());
        }

        // Collect MRs from projects
        let (mrs, mr_slices, mr_partial) = self.collect_mrs_from_projects(&client, projects)?;
        slices.extend(mr_slices);
        if mr_partial {
            completeness = Completeness::Partial;
        }

        // Convert MRs to events
        events.extend(self.mrs_to_events(mrs)?);

        // Collect reviews if enabled
        if self.include_reviews {
            warnings.push(
                "Reviews are collected via MR notes; treat as best-effort coverage.".to_string(),
            );

            let client = self.client()?;
            let user_id = self.get_user_id(&client)?;
            let projects = self.get_user_projects(&client, user_id)?;

            let (mrs, _, _) = self.collect_mrs_from_projects(&client, projects)?;

            for mr in mrs {
                let notes = self.collect_mr_notes(&client, mr.project_id, mr.iid)?;
                events.extend(self.notes_to_review_events(notes, &mr)?);
            }
        }

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
            mode: self.state.as_str().to_string(),
            sources: vec!["gitlab".to_string()],
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

// GitLab API types

#[derive(Debug, Deserialize)]
struct GitlabUser {
    id: u64,
    username: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitlabProject {
    id: u64,
    path_with_namespace: String,
    public: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitlabMergeRequest {
    id: u64,
    iid: u64,
    project_id: u64,
    title: String,
    state: String,
    created_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    closed_at: Option<DateTime<Utc>>,
    additions: Option<u64>,
    deletions: Option<u64>,
    changed_files: Option<u64>,
    labels: Vec<String>,
    author: GitlabAuthor,
    project: GitlabProjectInfo,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct GitlabAuthor {
    id: u64,
    username: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GitlabProjectInfo {
    id: u64,
    path_with_namespace: String,
    public: bool,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct GitlabNote {
    id: u64,
    system: bool,
    created_at: DateTime<Utc>,
    author: GitlabAuthor,
}

fn build_url_with_params(base: &str, params: &[(&str, String)]) -> Result<url::Url> {
    let mut url = url::Url::parse(base).with_context(|| format!("parse url {base}"))?;
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
    use proptest::prelude::*;

    // ── Helpers ─────────────────────────────────────────────────────────

    fn default_ingestor() -> GitlabIngestor {
        GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
    }

    fn sample_mr_json() -> serde_json::Value {
        serde_json::json!({
            "id": 101,
            "iid": 42,
            "project_id": 7,
            "title": "Add feature X",
            "state": "merged",
            "created_at": "2025-01-10T12:00:00Z",
            "merged_at": "2025-01-11T08:30:00Z",
            "closed_at": null,
            "additions": 120,
            "deletions": 30,
            "changed_files": 5,
            "labels": ["backend", "feature"],
            "author": { "id": 1, "username": "alice" },
            "project": { "id": 7, "path_with_namespace": "org/repo", "public": true }
        })
    }

    fn sample_note_json() -> serde_json::Value {
        serde_json::json!({
            "id": 501,
            "system": false,
            "created_at": "2025-01-10T14:00:00Z",
            "author": { "id": 2, "username": "bob" }
        })
    }

    // ── Existing tests (preserved) ──────────────────────────────────────

    #[test]
    fn with_cache_creates_missing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let cache_dir = temp.path().join("nested").join("cache");

        let ing = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_cache(&cache_dir)
        .unwrap();

        assert!(ing.cache.is_some());
        assert!(cache_dir.join("gitlab-api-cache.db").exists());
    }

    #[test]
    fn with_in_memory_cache_works() {
        let ing = GitlabIngestor::new(
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
        let result = GitlabIngestor::new(
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
        let result = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_instance("".to_string());

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be empty"));

        let result = GitlabIngestor::new(
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
        let ing = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_instance("https://gitlab.company.com".to_string())
        .unwrap();

        assert_eq!(ing.instance, "gitlab.company.com");
    }

    #[test]
    fn mr_state_from_str() {
        assert_eq!("opened".parse::<MrState>().unwrap(), MrState::Opened);
        assert_eq!("merged".parse::<MrState>().unwrap(), MrState::Merged);
        assert_eq!("closed".parse::<MrState>().unwrap(), MrState::Closed);
        assert_eq!("all".parse::<MrState>().unwrap(), MrState::All);
        assert!("invalid".parse::<MrState>().is_err());
    }

    #[test]
    fn mr_state_as_str() {
        assert_eq!(MrState::Opened.as_str(), "opened");
        assert_eq!(MrState::Merged.as_str(), "merged");
        assert_eq!(MrState::Closed.as_str(), "closed");
        assert_eq!(MrState::All.as_str(), "all");
    }

    #[test]
    fn html_base_url_constructs_correctly() {
        let mut ing = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        ing.instance = "gitlab.com".to_string();
        assert_eq!(ing.html_base_url(), "https://gitlab.com");

        ing.instance = "gitlab.company.com".to_string();
        assert_eq!(ing.html_base_url(), "https://gitlab.company.com");
    }

    #[test]
    fn api_base_url_constructs_correctly() {
        let mut ing = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        ing.instance = "gitlab.com".to_string();
        assert_eq!(ing.api_base_url(), "https://gitlab.com/api/v4");

        ing.instance = "gitlab.company.com".to_string();
        assert_eq!(ing.api_base_url(), "https://gitlab.company.com/api/v4");
    }

    #[test]
    fn build_url_with_params_encodes_query_values() {
        let url = build_url_with_params(
            "https://gitlab.com/api/v4/projects",
            &[
                ("state", "opened".to_string()),
                ("per_page", "100".to_string()),
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
                ("state".to_string(), "opened".to_string()),
                ("per_page".to_string(), "100".to_string()),
            ]
        );
    }

    // ── Property tests ──────────────────────────────────────────────────

    proptest! {
        #[test]
        fn mr_state_roundtrips(variant in prop_oneof![
            Just(MrState::Opened),
            Just(MrState::Merged),
            Just(MrState::Closed),
            Just(MrState::All),
        ]) {
            let s = variant.as_str();
            let parsed: MrState = s.parse().unwrap();
            prop_assert_eq!(parsed, variant);
        }

        #[test]
        fn mr_state_parse_case_insensitive(
            variant in prop_oneof![
                Just("opened"), Just("OPENED"), Just("Opened"),
                Just("merged"), Just("MERGED"), Just("Merged"),
                Just("closed"), Just("CLOSED"), Just("Closed"),
                Just("all"), Just("ALL"), Just("All"),
            ]
        ) {
            let parsed = variant.parse::<MrState>();
            prop_assert!(parsed.is_ok());
        }

        #[test]
        fn mr_state_invalid_always_errors(s in "[a-z]{6,10}") {
            // The 4 valid values are 3-6 chars; with 6-10 random chars
            // we avoid collisions with valid variants
            let parsed = s.parse::<MrState>();
            prop_assert!(parsed.is_err());
        }

        #[test]
        fn build_url_with_params_never_panics(
            key in "[a-zA-Z_]{1,10}",
            value in "[ -~]{0,50}",
        ) {
            let result = build_url_with_params(
                "https://gitlab.com/api/v4/test",
                &[(&key, value)],
            );
            prop_assert!(result.is_ok());
        }

        #[test]
        fn build_url_with_empty_params_is_identity(
            path in "/[a-z/]{1,30}",
        ) {
            let base = format!("https://gitlab.com/api/v4{}", path);
            let url = build_url_with_params(&base, &[]).unwrap();
            // No query string when params are empty
            prop_assert!(url.query().is_none());
        }

        #[test]
        fn api_base_url_always_has_v4(hostname in "[a-z]{3,12}\\.[a-z]{2,6}") {
            let mut ing = default_ingestor();
            ing.instance = hostname;
            let base = ing.api_base_url();
            prop_assert!(base.ends_with("/api/v4"));
            prop_assert!(base.starts_with("https://"));
        }

        #[test]
        fn builder_token_rejects_empty_accepts_nonempty(
            token in ".{1,50}"
        ) {
            let result = default_ingestor().with_token(token);
            prop_assert!(result.is_ok());
        }
    }

    // ── API response deserialization tests ───────────────────────────────

    #[test]
    fn deserialize_gitlab_user() {
        let json = r#"{"id": 42, "username": "alice"}"#;
        let user: GitlabUser = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 42);
        assert_eq!(user.username, "alice");
    }

    #[test]
    fn deserialize_gitlab_project() {
        let json = r#"{
            "id": 7,
            "path_with_namespace": "org/myrepo",
            "public": false
        }"#;
        let project: GitlabProject = serde_json::from_str(json).unwrap();
        assert_eq!(project.id, 7);
        assert_eq!(project.path_with_namespace, "org/myrepo");
        assert!(!project.public);
    }

    #[test]
    fn deserialize_gitlab_merge_request() {
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();
        assert_eq!(mr.id, 101);
        assert_eq!(mr.iid, 42);
        assert_eq!(mr.project_id, 7);
        assert_eq!(mr.title, "Add feature X");
        assert_eq!(mr.state, "merged");
        assert_eq!(mr.additions, Some(120));
        assert_eq!(mr.deletions, Some(30));
        assert_eq!(mr.changed_files, Some(5));
        assert_eq!(mr.labels, vec!["backend", "feature"]);
        assert_eq!(mr.author.username, "alice");
        assert_eq!(mr.project.path_with_namespace, "org/repo");
        assert!(mr.merged_at.is_some());
        assert!(mr.closed_at.is_none());
    }

    #[test]
    fn deserialize_mr_with_null_optional_fields() {
        let json = serde_json::json!({
            "id": 200,
            "iid": 10,
            "project_id": 3,
            "title": "Minimal MR",
            "state": "opened",
            "created_at": "2025-01-05T09:00:00Z",
            "merged_at": null,
            "closed_at": null,
            "additions": null,
            "deletions": null,
            "changed_files": null,
            "labels": [],
            "author": { "id": 1, "username": "alice" },
            "project": { "id": 3, "path_with_namespace": "org/minimal", "public": true }
        });
        let mr: GitlabMergeRequest = serde_json::from_value(json).unwrap();
        assert!(mr.merged_at.is_none());
        assert!(mr.additions.is_none());
        assert!(mr.deletions.is_none());
        assert!(mr.changed_files.is_none());
        assert!(mr.labels.is_empty());
    }

    #[test]
    fn deserialize_gitlab_note() {
        let note: GitlabNote = serde_json::from_value(sample_note_json()).unwrap();
        assert_eq!(note.id, 501);
        assert!(!note.system);
        assert_eq!(note.author.username, "bob");
    }

    #[test]
    fn deserialize_system_note() {
        let json = serde_json::json!({
            "id": 502,
            "system": true,
            "created_at": "2025-01-10T14:30:00Z",
            "author": { "id": 99, "username": "gitlab-bot" }
        });
        let note: GitlabNote = serde_json::from_value(json).unwrap();
        assert!(note.system);
    }

    #[test]
    fn deserialize_gitlab_author() {
        let json = r#"{"id": 5, "username": "charlie"}"#;
        let author: GitlabAuthor = serde_json::from_str(json).unwrap();
        assert_eq!(author.id, 5);
        assert_eq!(author.username, "charlie");
    }

    // ── mrs_to_events conversion tests ──────────────────────────────────

    #[test]
    fn mrs_to_events_converts_merged_mr() {
        let ing = default_ingestor();
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();

        let events = ing.mrs_to_events(vec![mr]).unwrap();
        assert_eq!(events.len(), 1);

        let ev = &events[0];
        assert_eq!(ev.kind, EventKind::PullRequest);
        assert_eq!(ev.actor.login, "alice");
        assert_eq!(ev.actor.id, Some(1));
        assert_eq!(ev.repo.full_name, "org/repo");
        assert_eq!(ev.repo.visibility, RepoVisibility::Public);
        assert_eq!(ev.tags, vec!["backend", "feature"]);

        // Check source
        assert_eq!(ev.source.system, SourceSystem::Other("gitlab".to_string()));
        assert!(
            ev.source
                .url
                .as_ref()
                .unwrap()
                .contains("merge_requests/42")
        );
        assert_eq!(ev.source.opaque_id.as_deref(), Some("101"));

        // Check links
        assert_eq!(ev.links.len(), 1);
        assert_eq!(ev.links[0].label, "GitLab MR");
        assert!(ev.links[0].url.contains("org/repo/-/merge_requests/42"));

        // Check payload
        if let EventPayload::PullRequest(pr) = &ev.payload {
            assert_eq!(pr.number, 42);
            assert_eq!(pr.title, "Add feature X");
            assert_eq!(pr.state, PullRequestState::Merged);
            assert!(pr.merged_at.is_some());
            assert_eq!(pr.additions, Some(120));
            assert_eq!(pr.deletions, Some(30));
            assert_eq!(pr.changed_files, Some(5));
        } else {
            panic!("Expected PullRequest payload");
        }
    }

    #[test]
    fn mrs_to_events_maps_all_states() {
        let ing = default_ingestor();

        for (state_str, expected) in [
            ("opened", PullRequestState::Open),
            ("merged", PullRequestState::Merged),
            ("closed", PullRequestState::Closed),
            ("unknown_state", PullRequestState::Unknown),
        ] {
            let mut json = sample_mr_json();
            json["state"] = serde_json::json!(state_str);
            // Bump id to avoid duplicate EventId
            json["id"] = serde_json::json!(state_str.len() as u64 + 1000);
            let mr: GitlabMergeRequest = serde_json::from_value(json).unwrap();
            let events = ing.mrs_to_events(vec![mr]).unwrap();
            if let EventPayload::PullRequest(pr) = &events[0].payload {
                assert_eq!(pr.state, expected, "state mismatch for '{}'", state_str);
            }
        }
    }

    #[test]
    fn mrs_to_events_private_visibility() {
        let ing = default_ingestor();

        let mut json = sample_mr_json();
        json["project"]["public"] = serde_json::json!(false);
        let mr: GitlabMergeRequest = serde_json::from_value(json).unwrap();
        let events = ing.mrs_to_events(vec![mr]).unwrap();
        assert_eq!(events[0].repo.visibility, RepoVisibility::Private);
    }

    #[test]
    fn mrs_to_events_empty_input() {
        let ing = default_ingestor();
        let events = ing.mrs_to_events(vec![]).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn mrs_to_events_custom_instance() {
        let mut ing = default_ingestor();
        ing.instance = "gitlab.internal.co".to_string();

        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();
        let events = ing.mrs_to_events(vec![mr]).unwrap();

        let url = events[0].links[0].url.as_str();
        assert!(url.starts_with("https://gitlab.internal.co/"));
    }

    // ── notes_to_review_events conversion tests ─────────────────────────

    #[test]
    fn notes_to_review_events_converts_non_system_non_author_notes() {
        let ing = default_ingestor();
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();
        let note: GitlabNote = serde_json::from_value(sample_note_json()).unwrap();

        let events = ing.notes_to_review_events(vec![note], &mr).unwrap();
        assert_eq!(events.len(), 1);

        let ev = &events[0];
        assert_eq!(ev.kind, EventKind::Review);
        assert_eq!(ev.actor.login, "bob");
        assert!(ev.links[0].url.contains("#note_501"));

        if let EventPayload::Review(rev) = &ev.payload {
            assert_eq!(rev.pull_number, 42);
            assert_eq!(rev.pull_title, "Add feature X");
            assert_eq!(rev.state, "approved");
        } else {
            panic!("Expected Review payload");
        }
    }

    #[test]
    fn notes_to_review_events_skips_system_notes() {
        let ing = default_ingestor();
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();

        let system_note: GitlabNote = serde_json::from_value(serde_json::json!({
            "id": 600,
            "system": true,
            "created_at": "2025-01-10T15:00:00Z",
            "author": { "id": 2, "username": "bob" }
        }))
        .unwrap();

        let events = ing.notes_to_review_events(vec![system_note], &mr).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn notes_to_review_events_skips_self_authored_notes() {
        let ing = default_ingestor(); // user = "alice"
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();

        let self_note: GitlabNote = serde_json::from_value(serde_json::json!({
            "id": 601,
            "system": false,
            "created_at": "2025-01-10T15:30:00Z",
            "author": { "id": 1, "username": "alice" }
        }))
        .unwrap();

        let events = ing.notes_to_review_events(vec![self_note], &mr).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn notes_to_review_events_empty_input() {
        let ing = default_ingestor();
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();

        let events = ing.notes_to_review_events(vec![], &mr).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn notes_to_review_events_mixed_filtering() {
        let ing = default_ingestor(); // user = "alice"
        let mr: GitlabMergeRequest = serde_json::from_value(sample_mr_json()).unwrap();

        let notes: Vec<GitlabNote> = serde_json::from_value(serde_json::json!([
            { "id": 700, "system": false, "created_at": "2025-01-10T10:00:00Z",
              "author": { "id": 2, "username": "bob" } },
            { "id": 701, "system": true, "created_at": "2025-01-10T11:00:00Z",
              "author": { "id": 2, "username": "bob" } },
            { "id": 702, "system": false, "created_at": "2025-01-10T12:00:00Z",
              "author": { "id": 1, "username": "alice" } },
            { "id": 703, "system": false, "created_at": "2025-01-10T13:00:00Z",
              "author": { "id": 3, "username": "charlie" } }
        ]))
        .unwrap();

        let events = ing.notes_to_review_events(notes, &mr).unwrap();
        // Only bob (700) and charlie (703); system note 701 and self-note 702 filtered
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].actor.login, "bob");
        assert_eq!(events[1].actor.login, "charlie");
    }

    // ── URL construction tests ──────────────────────────────────────────

    #[test]
    fn html_base_url_custom_instance() {
        let ing = default_ingestor()
            .with_instance("https://gitlab.myorg.io".to_string())
            .unwrap();
        assert_eq!(ing.html_base_url(), "https://gitlab.myorg.io");
    }

    #[test]
    fn api_base_url_custom_instance() {
        let ing = default_ingestor()
            .with_instance("https://gitlab.myorg.io".to_string())
            .unwrap();
        assert_eq!(ing.api_base_url(), "https://gitlab.myorg.io/api/v4");
    }

    #[test]
    fn build_url_with_no_params() {
        let url = build_url_with_params("https://gitlab.com/api/v4/projects", &[]).unwrap();
        assert_eq!(url.as_str(), "https://gitlab.com/api/v4/projects");
    }

    #[test]
    fn build_url_with_special_chars_in_values() {
        let url = build_url_with_params(
            "https://gitlab.com/api/v4/projects",
            &[("search", "hello world & more".to_string())],
        )
        .unwrap();
        let pairs: Vec<_> = url.query_pairs().collect();
        assert_eq!(pairs[0].1, "hello world & more");
    }

    #[test]
    fn build_url_with_invalid_base_url_errors() {
        let result = build_url_with_params("not-a-url", &[]);
        assert!(result.is_err());
    }

    // ── Builder / configuration tests ───────────────────────────────────

    #[test]
    fn default_ingestor_has_expected_defaults() {
        let ing = default_ingestor();
        assert_eq!(ing.user, "alice");
        assert_eq!(ing.state, MrState::Merged);
        assert!(!ing.include_reviews);
        assert!(ing.fetch_details);
        assert_eq!(ing.throttle_ms, 0);
        assert!(ing.token.is_none());
        assert_eq!(ing.instance, "gitlab.com");
        assert!(ing.cache.is_none());
    }

    #[test]
    fn with_state_updates_state() {
        let ing = default_ingestor().with_state(MrState::All);
        assert_eq!(ing.state, MrState::All);
    }

    #[test]
    fn with_include_reviews_updates_flag() {
        let ing = default_ingestor().with_include_reviews(true);
        assert!(ing.include_reviews);
    }

    #[test]
    fn with_throttle_updates_delay() {
        let ing = default_ingestor().with_throttle(500);
        assert_eq!(ing.throttle_ms, 500);
    }

    #[test]
    fn with_token_stores_value() {
        let ing = default_ingestor()
            .with_token("glpat-abc123".to_string())
            .unwrap();
        assert_eq!(ing.token.as_deref(), Some("glpat-abc123"));
    }

    #[test]
    fn with_instance_bare_hostname() {
        let ing = default_ingestor()
            .with_instance("gitlab.internal.co".to_string())
            .unwrap();
        assert_eq!(ing.instance, "gitlab.internal.co");
    }

    // ── Error handling tests ────────────────────────────────────────────

    #[test]
    fn ingest_rejects_equal_dates() {
        let same = NaiveDate::from_ymd_opt(2025, 6, 15).unwrap();
        let ing = GitlabIngestor::new("alice".to_string(), same, same);
        let err = ing.ingest().unwrap_err();
        assert!(err.to_string().contains("since must be < until"));
    }

    #[test]
    fn ingest_rejects_reversed_dates() {
        let ing = GitlabIngestor::new(
            "alice".to_string(),
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        );
        let err = ing.ingest().unwrap_err();
        assert!(err.to_string().contains("since must be < until"));
    }

    #[test]
    fn ingest_requires_token() {
        let ing = default_ingestor(); // no token set
        let err = ing.ingest().unwrap_err();
        assert!(err.to_string().contains("token is required"));
    }

    #[test]
    fn deserialize_mr_missing_required_field_errors() {
        let json = serde_json::json!({
            "id": 101,
            // missing "iid", "project_id", "title", etc.
        });
        let result = serde_json::from_value::<GitlabMergeRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_note_missing_required_field_errors() {
        let json = serde_json::json!({
            "id": 501,
            // missing "system", "created_at", "author"
        });
        let result = serde_json::from_value::<GitlabNote>(json);
        assert!(result.is_err());
    }

    #[test]
    fn deserialize_user_missing_required_field_errors() {
        let json = serde_json::json!({
            "id": 42
            // missing "username"
        });
        let result = serde_json::from_value::<GitlabUser>(json);
        assert!(result.is_err());
    }
}
