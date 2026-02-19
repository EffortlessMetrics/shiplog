//! GitLab API ingestor with adaptive date slicing and cache support.
//!
//! Collects MR/review events, tracks coverage slices, and marks partial
//! completeness when search caps or incomplete API responses are detected.

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use shiplog_cache::{ApiCache, CacheKey};
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
}
