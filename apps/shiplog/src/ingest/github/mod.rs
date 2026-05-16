//! GitHub API ingestor with adaptive date slicing and cache support.
//!
//! Collects PR/review events, tracks coverage slices, and marks partial
//! completeness when search caps or incomplete API responses are detected.

mod pr_events;

use crate::cache::{ApiCache, CacheKey, CacheLookup};
use crate::coverage::{day_windows, month_windows, week_windows, window_len_days};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use shiplog::ids::{EventId, RunId};
use shiplog::ports::{IngestOutput, Ingestor};
use shiplog::schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog::schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, RepoRef, RepoVisibility, ReviewEvent,
    SourceRef, SourceSystem,
};
use shiplog::schema::freshness::{FreshnessStatus, SourceFreshness};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::sleep;
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub struct GithubIngestor {
    pub user: String,
    pub since: NaiveDate,
    pub until: NaiveDate,
    /// "merged" or "created"
    pub mode: String,
    pub include_reviews: bool,
    pub fetch_details: bool,
    pub throttle_ms: u64,
    pub token: Option<String>,
    /// GitHub API base URL (for GHES). Default: <https://api.github.com>
    pub api_base: String,
    /// Optional cache for API responses
    pub cache: Option<ApiCache>,
    /// Adapter-local cache hit counter for the most recent (or
    /// in-progress) `ingest()` call. Incremented every time
    /// `self.cache.get(...)` returns `Some(_)`. Reported in the
    /// per-source [`SourceFreshness`] receipt at end of ingest. Private
    /// because the counter is a run-scoped diagnostic, not a public API.
    cache_hits: AtomicU64,
    /// Adapter-local cache miss counter. Incremented every time
    /// `self.cache.get(...)` returns `None` and triggers a fresh fetch.
    /// Equivalent to "live API calls performed under the cache". See
    /// [`Self::cache_hits`] above.
    cache_misses: AtomicU64,
    /// Adapter-local stale-hit counter. Incremented when `ApiCache::lookup`
    /// returns an expired row that this adapter uses.
    cache_stale_hits: AtomicU64,
}

impl GithubIngestor {
    /// Create a new GitHub ingestor for the given user and date range.
    ///
    /// Defaults to `merged` mode with no reviews, no cache, and no throttle.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog::ingest::github::GithubIngestor;
    /// use chrono::NaiveDate;
    ///
    /// let ingestor = GithubIngestor::new(
    ///     "octocat".into(),
    ///     NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    /// );
    /// assert_eq!(ingestor.mode, "merged");
    /// ```
    pub fn new(user: String, since: NaiveDate, until: NaiveDate) -> Self {
        Self {
            user,
            since,
            until,
            mode: "merged".to_string(),
            include_reviews: false,
            fetch_details: true,
            throttle_ms: 0,
            token: None,
            api_base: "https://api.github.com".to_string(),
            cache: None,
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_stale_hits: AtomicU64::new(0),
        }
    }

    /// Enable caching with the given cache directory.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use shiplog::ingest::github::GithubIngestor;
    /// use chrono::NaiveDate;
    ///
    /// let ingestor = GithubIngestor::new(
    ///     "octocat".into(),
    ///     NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    /// ).with_cache("./cache")?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn with_cache(mut self, cache_dir: impl Into<PathBuf>) -> Result<Self> {
        let cache_path = cache_dir.into().join("github-api-cache.db");
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create GitHub cache directory {parent:?}"))?;
        }
        let cache = ApiCache::open(&cache_path)
            .with_context(|| format!("open GitHub API cache at {cache_path:?}"))?;
        self.cache = Some(cache);
        Ok(self)
    }

    /// Enable in-memory caching (useful for testing).
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog::ingest::github::GithubIngestor;
    /// use chrono::NaiveDate;
    ///
    /// let ingestor = GithubIngestor::new(
    ///     "octocat".into(),
    ///     NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ///     NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    /// ).with_in_memory_cache().unwrap();
    /// assert!(ingestor.cache.is_some());
    /// ```
    pub fn with_in_memory_cache(mut self) -> Result<Self> {
        let cache = ApiCache::open_in_memory().context("open in-memory API cache")?;
        self.cache = Some(cache);
        Ok(self)
    }

    fn html_base_url(&self) -> String {
        if let Ok(u) = Url::parse(&self.api_base) {
            let scheme = u.scheme();
            if let Some(host) = u.host_str() {
                if host == "api.github.com" {
                    return "https://github.com".to_string();
                }
                let port_suffix = u.port().map(|p| format!(":{p}")).unwrap_or_default();
                return format!("{scheme}://{host}{port_suffix}");
            }
        }
        "https://github.com".to_string()
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
        format!("{}{}", self.api_base.trim_end_matches('/'), path)
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

        let mut req = client
            .get(request_url)
            .header("Accept", "application/vnd.github+json");
        req = req.header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .with_context(|| format!("GET {request_url_for_err}"))?;
        self.throttle();

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(anyhow!("GitHub API error {status}: {body}"));
        }

        resp.json::<T>()
            .with_context(|| format!("parse json from {request_url_for_err}"))
    }
}

impl Ingestor for GithubIngestor {
    #[mutants::skip]
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let client = self.client().context("create GitHub API client")?;
        let run_id = RunId::now("shiplog");
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut completeness = Completeness::Complete;

        let mut events: Vec<EventEnvelope> = Vec::new();

        // PRs authored
        let pr_query_builder = |w: &TimeWindow| self.build_pr_query(w);
        let (pr_items, pr_slices, pr_partial) =
            self.collect_search_items(&client, pr_query_builder, self.since, self.until, "prs")?;
        slices.extend(pr_slices);
        if pr_partial {
            completeness = Completeness::Partial;
        }

        events.extend(self.items_to_pr_events(&client, pr_items)?);

        // Reviews authored (best-effort)
        if self.include_reviews {
            warnings.push("Reviews are collected via search + per-PR review fetch; treat as best-effort coverage.".to_string());
            let review_query_builder = |w: &TimeWindow| self.build_reviewed_query(w);
            let (review_items, review_slices, review_partial) = self.collect_search_items(
                &client,
                review_query_builder,
                self.since,
                self.until,
                "reviews",
            )?;
            slices.extend(review_slices);
            if review_partial {
                completeness = Completeness::Partial;
            }
            events.extend(self.items_to_review_events(&client, review_items)?);
        }

        // Sort for stable output
        events.sort_by_key(|e| e.occurred_at);

        let fetched_at = Utc::now();
        let cov = CoverageManifest {
            run_id,
            generated_at: fetched_at,
            user: self.user.clone(),
            window: TimeWindow {
                since: self.since,
                until: self.until,
            },
            mode: self.mode.clone(),
            sources: vec!["github".to_string()],
            slices,
            warnings,
            completeness,
        };

        // Snapshot the run's cache counters and derive freshness status.
        // Status rules:
        //   - cache configured and >0 hits and 0 misses => Cached.
        //   - cache configured and >=1 miss (regardless of hits) => Fresh.
        //   - cache configured and 0 hits + 0 misses => Fresh (the search
        //     phase always runs uncached, so the adapter performed live
        //     work even if `fetch_details=false` skipped the detail leg).
        //   - cache not configured => Fresh.
        let hits = self.cache_hits.load(Ordering::Relaxed);
        let misses = self.cache_misses.load(Ordering::Relaxed);
        let stale_hits = self.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(self.cache.is_some(), hits, misses, stale_hits);
        let freshness = vec![SourceFreshness {
            source: "github".to_string(),
            status,
            cache_hits: hits,
            cache_misses: misses,
            fetched_at: Some(fetched_at),
            reason,
        }];

        Ok(IngestOutput {
            events,
            coverage: cov,
            freshness,
        })
    }
}

impl GithubIngestor {
    fn build_pr_query(&self, w: &TimeWindow) -> String {
        let (start, end) = github_inclusive_range(w);
        match self.mode.as_str() {
            "created" => format!("is:pr author:{} created:{}..{}", self.user, start, end),
            _ => format!(
                "is:pr is:merged author:{} merged:{}..{}",
                self.user, start, end
            ),
        }
    }

    fn build_reviewed_query(&self, w: &TimeWindow) -> String {
        // GitHub does not expose review submission time in search qualifiers.
        // We use `updated:` to find candidate PRs, then filter reviews by submitted_at.
        let (start, end) = github_inclusive_range(w);
        format!("is:pr reviewed-by:{} updated:{}..{}", self.user, start, end)
    }

    /// Collect search items for a date range, adaptively slicing to avoid the 1000-result cap.
    ///
    /// Returns:
    /// - items
    /// - coverage slices
    /// - whether coverage is partial
    #[mutants::skip]
    fn collect_search_items<F>(
        &self,
        client: &Client,
        make_query: F,
        since: NaiveDate,
        until: NaiveDate,
        label: &str,
    ) -> Result<(Vec<SearchIssueItem>, Vec<CoverageSlice>, bool)>
    where
        F: Fn(&TimeWindow) -> String,
    {
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let mut items: Vec<SearchIssueItem> = Vec::new();
        let mut partial = false;

        for w in month_windows(since, until) {
            let (mut i, mut s, p) =
                self.collect_window(client, &make_query, &w, Granularity::Month, label)?;
            items.append(&mut i);
            slices.append(&mut s);
            partial |= p;
        }

        Ok((items, slices, partial))
    }

    #[mutants::skip]
    fn collect_window<F>(
        &self,
        client: &Client,
        make_query: &F,
        window: &TimeWindow,
        gran: Granularity,
        label: &str,
    ) -> Result<(Vec<SearchIssueItem>, Vec<CoverageSlice>, bool)>
    where
        F: Fn(&TimeWindow) -> String,
    {
        if window.since >= window.until {
            return Ok((vec![], vec![], false));
        }

        let query = make_query(window);
        let (meta_total, meta_incomplete) = self.search_meta(client, &query)?;
        let mut slices = vec![CoverageSlice {
            window: window.clone(),
            query: query.clone(),
            total_count: meta_total,
            fetched: 0,
            incomplete_results: Some(meta_incomplete),
            notes: vec![format!("probe:{label}")],
        }];

        // Decide if we need to subdivide
        let need_subdivide = meta_total > 1000 || meta_incomplete;
        let can_subdivide = gran != Granularity::Day && window_len_days(window) > 1;

        if need_subdivide && can_subdivide {
            slices[0].notes.push(format!(
                "subdivide:{}",
                if meta_total > 1000 {
                    "cap"
                } else {
                    "incomplete"
                }
            ));

            let mut out_items = Vec::new();
            let mut out_slices = slices;
            let mut partial = false;

            let subs = match gran {
                Granularity::Month => week_windows(window.since, window.until),
                Granularity::Week => day_windows(window.since, window.until),
                Granularity::Day => vec![],
            };

            for sub in subs {
                let (mut i, mut s, p) =
                    self.collect_window(client, make_query, &sub, gran.next(), label)?;
                out_items.append(&mut i);
                out_slices.append(&mut s);
                partial |= p;
            }
            return Ok((out_items, out_slices, partial));
        }

        // Day-level overflow: can't subdivide further. We'll still fetch up to the API cap.
        let mut partial = false;
        if meta_total > 1000 || meta_incomplete {
            partial = true;
            slices[0]
                .notes
                .push("partial:unresolvable_at_this_granularity".to_string());
        }

        let fetched_items = self.fetch_all_search_items(client, &query)?;
        let fetched = fetched_items.len() as u64;

        // Record a fetch slice (separate from the probe for clarity)
        slices.push(CoverageSlice {
            window: window.clone(),
            query: query.clone(),
            total_count: meta_total,
            fetched,
            incomplete_results: Some(meta_incomplete),
            notes: vec![format!("fetch:{label}")],
        });

        Ok((fetched_items, slices, partial))
    }

    #[mutants::skip]
    fn search_meta(&self, client: &Client, q: &str) -> Result<(u64, bool)> {
        let url = self.api_url("/search/issues");
        let resp: SearchResponse<SearchIssueItem> = self.get_json(
            client,
            &url,
            &[
                ("q", q.to_string()),
                ("per_page", "1".to_string()),
                ("page", "1".to_string()),
            ],
        )?;
        Ok((resp.total_count, resp.incomplete_results))
    }

    #[mutants::skip]
    fn fetch_all_search_items(&self, client: &Client, q: &str) -> Result<Vec<SearchIssueItem>> {
        let url = self.api_url("/search/issues");
        let mut out: Vec<SearchIssueItem> = Vec::new();
        let per_page = 100;
        let max_pages = 10; // 1000 cap
        for page in 1..=max_pages {
            let resp: SearchResponse<SearchIssueItem> = self.get_json(
                client,
                &url,
                &[
                    ("q", q.to_string()),
                    ("per_page", per_page.to_string()),
                    ("page", page.to_string()),
                ],
            )?;
            let items_len = resp.items.len();
            out.extend(resp.items);
            if out.len() as u64 >= resp.total_count.min(1000) {
                break;
            }
            if items_len < per_page {
                break;
            }
        }
        Ok(out)
    }

    #[mutants::skip]
    fn items_to_pr_events(
        &self,
        client: &Client,
        items: Vec<SearchIssueItem>,
    ) -> Result<Vec<EventEnvelope>> {
        pr_events::items_to_pr_events(self, client, items)
    }

    #[mutants::skip]
    fn items_to_review_events(
        &self,
        client: &Client,
        items: Vec<SearchIssueItem>,
    ) -> Result<Vec<EventEnvelope>> {
        let mut out = Vec::new();
        for item in items {
            let Some(pr_ref) = &item.pull_request else {
                continue;
            };
            let html_base = self.html_base_url();
            let (repo_full_name, repo_html_url) =
                repo_from_repo_url(&item.repository_url, &html_base);

            // Fetch reviews for this PR and filter by author + date window.
            let reviews = self.fetch_pr_reviews(client, &pr_ref.url)?;
            for r in reviews {
                if r.user.login != self.user {
                    continue;
                }
                let submitted = match r.submitted_at {
                    Some(s) => s,
                    None => continue,
                };
                let submitted_date = submitted.date_naive();
                if submitted_date < self.since || submitted_date >= self.until {
                    continue;
                }

                let id = EventId::from_parts([
                    "github",
                    "review",
                    &repo_full_name,
                    &item.number.to_string(),
                    &r.id.to_string(),
                ]);

                let ev = EventEnvelope {
                    id,
                    kind: EventKind::Review,
                    occurred_at: submitted,
                    actor: Actor {
                        login: self.user.clone(),
                        id: None,
                    },
                    repo: RepoRef {
                        full_name: repo_full_name.clone(),
                        html_url: Some(repo_html_url.clone()),
                        visibility: RepoVisibility::Unknown,
                    },
                    payload: EventPayload::Review(ReviewEvent {
                        pull_number: item.number,
                        pull_title: item.title.clone(),
                        submitted_at: submitted,
                        state: r.state,
                        window: None,
                    }),
                    tags: vec![],
                    links: vec![Link {
                        label: "pr".into(),
                        url: item.html_url.clone(),
                    }],
                    source: SourceRef {
                        system: SourceSystem::Github,
                        url: Some(pr_ref.url.clone()),
                        opaque_id: Some(r.id.to_string()),
                    },
                };

                out.push(ev);
            }
        }
        Ok(out)
    }

    #[mutants::skip]
    fn fetch_pr_details(&self, client: &Client, pr_api_url: &str) -> Result<PullRequestDetails> {
        // Check cache first
        let cache_key = CacheKey::pr_details(pr_api_url);
        if let Some(ref cache) = self.cache {
            match cache.lookup::<PullRequestDetails>(&cache_key)? {
                CacheLookup::Fresh(cached) => {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(cached);
                }
                CacheLookup::Stale(cached) => {
                    self.cache_hits.fetch_add(1, Ordering::Relaxed);
                    self.cache_stale_hits.fetch_add(1, Ordering::Relaxed);
                    return Ok(cached);
                }
                CacheLookup::Miss => {
                    self.cache_misses.fetch_add(1, Ordering::Relaxed);
                }
            }
        }

        // Fetch from API
        let details: PullRequestDetails = self.get_json(client, pr_api_url, &[])?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            cache.set(&cache_key, &details)?;
        }

        Ok(details)
    }

    #[mutants::skip]
    fn fetch_pr_reviews(
        &self,
        client: &Client,
        pr_api_url: &str,
    ) -> Result<Vec<PullRequestReview>> {
        let url = format!("{pr_api_url}/reviews");
        let mut out = Vec::new();
        let per_page = 100;
        for page in 1..=10 {
            let cache_key = CacheKey::pr_reviews(pr_api_url, page);

            // Try to get from cache first
            let page_reviews: Vec<PullRequestReview> = if let Some(ref cache) = self.cache {
                match cache.lookup::<Vec<PullRequestReview>>(&cache_key)? {
                    CacheLookup::Fresh(cached) => {
                        self.cache_hits.fetch_add(1, Ordering::Relaxed);
                        cached
                    }
                    CacheLookup::Stale(cached) => {
                        self.cache_hits.fetch_add(1, Ordering::Relaxed);
                        self.cache_stale_hits.fetch_add(1, Ordering::Relaxed);
                        cached
                    }
                    CacheLookup::Miss => {
                        self.cache_misses.fetch_add(1, Ordering::Relaxed);
                        // Not in cache, fetch from API
                        let reviews: Vec<PullRequestReview> = self.get_json(
                            client,
                            &url,
                            &[
                                ("per_page", per_page.to_string()),
                                ("page", page.to_string()),
                            ],
                        )?;
                        // Store in cache
                        cache.set(&cache_key, &reviews)?;
                        reviews
                    }
                }
            } else {
                // No cache configured, fetch directly
                self.get_json(
                    client,
                    &url,
                    &[
                        ("per_page", per_page.to_string()),
                        ("page", page.to_string()),
                    ],
                )?
            };

            let n = page_reviews.len();
            out.extend(page_reviews);
            if n < per_page {
                break;
            }
        }
        Ok(out)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Granularity {
    Month,
    Week,
    Day,
}

impl Granularity {
    fn next(&self) -> Granularity {
        match self {
            Granularity::Month => Granularity::Week,
            Granularity::Week => Granularity::Day,
            Granularity::Day => Granularity::Day,
        }
    }
}

fn github_inclusive_range(w: &TimeWindow) -> (String, String) {
    let start = w.since.format("%Y-%m-%d").to_string();
    let end_date = w.until.pred_opt().unwrap_or(w.until);
    let end = end_date.format("%Y-%m-%d").to_string();
    (start, end)
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

fn github_freshness_status(
    cache_configured: bool,
    cache_hits: u64,
    cache_misses: u64,
    cache_stale_hits: u64,
) -> (FreshnessStatus, Option<String>) {
    if cache_configured && cache_stale_hits > 0 {
        return (
            FreshnessStatus::Stale,
            Some("one or more expired cache entries were used".to_string()),
        );
    }
    if cache_configured && cache_hits > 0 && cache_misses == 0 {
        return (FreshnessStatus::Cached, None);
    }

    (FreshnessStatus::Fresh, None)
}

fn repo_from_repo_url(repo_api_url: &str, html_base: &str) -> (String, String) {
    #[expect(clippy::collapsible_if, reason = "policy:clippy-0002")]
    if let Ok(u) = Url::parse(repo_api_url) {
        if let Some(segs) = u.path_segments() {
            let v: Vec<&str> = segs.collect();
            if v.len() >= 3 && v[0] == "repos" {
                let owner = v[1];
                let repo = v[2];
                let full = format!("{}/{}", owner, repo);
                let html = format!("{}/{}/{}", html_base.trim_end_matches('/'), owner, repo);
                return (full, html);
            }
        }
    }
    ("unknown/unknown".to_string(), html_base.to_string())
}

/// GitHub search response envelope.
#[derive(Debug, Deserialize)]
struct SearchResponse<T> {
    total_count: u64,
    incomplete_results: bool,
    items: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct SearchIssueItem {
    id: u64,
    number: u64,
    title: String,
    html_url: String,
    repository_url: String,
    pull_request: Option<SearchPullRequestRef>,

    // Search returns these for issues; for PR queries they are present and useful.
    created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct SearchPullRequestRef {
    url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PullRequestDetails {
    title: String,
    created_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    additions: u64,
    deletions: u64,
    changed_files: u64,
    base: PullBase,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PullBase {
    repo: PullRepo,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PullRepo {
    full_name: String,
    html_url: String,
    #[serde(rename = "private")]
    private_field: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct PullRequestReview {
    id: u64,
    state: String,
    submitted_at: Option<DateTime<Utc>>,
    user: ReviewUser,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ReviewUser {
    login: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use proptest::prelude::*;
    use shiplog::schema::event::PullRequestState;
    use std::io::{ErrorKind, Read, Write};
    use std::net::{SocketAddr, TcpListener, TcpStream};
    use std::sync::{Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::{Duration as StdDuration, Instant};

    // ── helpers ──────────────────────────────────────────────────────────

    fn make_ingestor(user: &str) -> GithubIngestor {
        GithubIngestor::new(
            user.to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
    }

    fn make_search_item(number: u64, repo: &str, with_pr: bool) -> SearchIssueItem {
        SearchIssueItem {
            id: number * 100,
            number,
            title: format!("PR #{number}"),
            html_url: format!("https://github.com/{repo}/pull/{number}"),
            repository_url: format!("https://api.github.com/repos/{repo}"),
            pull_request: if with_pr {
                Some(SearchPullRequestRef {
                    url: format!("https://api.github.com/repos/{repo}/pulls/{number}"),
                })
            } else {
                None
            },
            created_at: Some(Utc::now()),
        }
    }

    // ── existing tests (preserved) ──────────────────────────────────────

    #[test]
    fn with_cache_creates_missing_directory() {
        let temp = tempfile::tempdir().unwrap();
        let cache_dir = temp.path().join("nested").join("cache");

        let ing = GithubIngestor::new(
            "octocat".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        )
        .with_cache(&cache_dir)
        .unwrap();

        assert!(ing.cache.is_some());
        assert!(cache_dir.join("github-api-cache.db").exists());
    }

    #[test]
    fn build_url_with_params_encodes_query_values() {
        let url = build_url_with_params(
            "https://api.github.com/search/issues",
            &[
                ("q", "is:pr is:merged author:octocat".to_string()),
                ("per_page", "1".to_string()),
            ],
        )
        .unwrap();

        assert!(!url.as_str().contains(' '), "URL should be percent-encoded");

        let pairs: Vec<(String, String)> = url
            .query_pairs()
            .map(|(k, v)| (k.into_owned(), v.into_owned()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                (
                    "q".to_string(),
                    "is:pr is:merged author:octocat".to_string()
                ),
                ("per_page".to_string(), "1".to_string()),
            ]
        );
    }

    #[test]
    fn github_inclusive_range_uses_exclusive_until_date() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };

        let (start, end) = github_inclusive_range(&window);
        assert_eq!(start, "2025-01-01");
        assert_eq!(end, "2025-01-31");
    }

    #[test]
    fn html_base_url_maps_public_and_ghes_hosts() {
        let mut ing = GithubIngestor::new(
            "octocat".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        ing.api_base = "https://api.github.com".to_string();
        assert_eq!(ing.html_base_url(), "https://github.com");

        ing.api_base = "https://github.enterprise.local/api/v3".to_string();
        assert_eq!(ing.html_base_url(), "https://github.enterprise.local");
    }

    #[test]
    fn build_pr_query_merged_and_created_modes() {
        let mut ing = GithubIngestor::new(
            "octocat".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };

        ing.mode = "merged".to_string();
        let merged_q = ing.build_pr_query(&w);
        assert!(!merged_q.is_empty());
        assert!(merged_q.contains("is:merged"));
        assert!(merged_q.contains("author:octocat"));

        ing.mode = "created".to_string();
        let created_q = ing.build_pr_query(&w);
        assert!(!created_q.is_empty());
        assert!(created_q.contains("created:"));
        assert!(created_q.contains("author:octocat"));

        // The two queries should be different
        assert_ne!(merged_q, created_q);
    }

    #[test]
    fn build_reviewed_query_contains_user() {
        let ing = GithubIngestor::new(
            "octocat".to_string(),
            NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        );
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };

        let q = ing.build_reviewed_query(&w);
        assert!(!q.is_empty());
        assert!(q.contains("reviewed-by:octocat"));
        assert!(q.contains("is:pr"));
    }

    #[test]
    fn repo_from_repo_url_invalid_url_returns_fallback() {
        let (full, html) = repo_from_repo_url("not-a-url-at-all", "https://github.com");
        assert_eq!(full, "unknown/unknown");
        assert_eq!(html, "https://github.com");

        // URL with wrong path structure
        let (full2, _) =
            repo_from_repo_url("https://api.github.com/users/octocat", "https://github.com");
        assert_eq!(full2, "unknown/unknown");
    }

    #[test]
    fn repo_from_repo_url_extracts_or_falls_back() {
        let (full, html) = repo_from_repo_url(
            "https://api.github.com/repos/owner/repo",
            "https://github.com",
        );
        assert_eq!(full, "owner/repo");
        assert_eq!(html, "https://github.com/owner/repo");

        let (full_fallback, html_fallback) = repo_from_repo_url("not-a-url", "https://github.com");
        assert_eq!(full_fallback, "unknown/unknown");
        assert_eq!(html_fallback, "https://github.com");
    }

    // ── new unit tests ──────────────────────────────────────────────────

    // -- Granularity --

    #[test]
    fn granularity_next_transitions() {
        assert_eq!(Granularity::Month.next(), Granularity::Week);
        assert_eq!(Granularity::Week.next(), Granularity::Day);
        assert_eq!(Granularity::Day.next(), Granularity::Day);
    }

    #[test]
    fn granularity_day_is_fixed_point() {
        let g = Granularity::Day;
        assert_eq!(g.next(), Granularity::Day);
        assert_eq!(g.next().next(), Granularity::Day);
    }

    // -- GithubIngestor::new defaults --

    #[test]
    fn new_defaults_are_correct() {
        let ing = make_ingestor("alice");
        assert_eq!(ing.user, "alice");
        assert_eq!(ing.mode, "merged");
        assert!(!ing.include_reviews);
        assert!(ing.fetch_details);
        assert_eq!(ing.throttle_ms, 0);
        assert!(ing.token.is_none());
        assert_eq!(ing.api_base, "https://api.github.com");
        assert!(ing.cache.is_none());
    }

    // -- with_in_memory_cache --

    #[test]
    fn with_in_memory_cache_sets_cache() {
        let ing = make_ingestor("bob").with_in_memory_cache().unwrap();
        assert!(ing.cache.is_some());
    }

    // -- cache hit/miss counters --
    //
    // Exercises the freshness-attribution wiring without involving the
    // network. `fetch_pr_details` is the canonical cache-aware path; a
    // miss followed by a hit on the same key should leave the counters
    // at (1 miss, 1 hit), and the second call must return the cached
    // value rather than re-fetching it (verified by storing a sentinel
    // value in the cache manually).

    #[test]
    fn freshness_counters_start_at_zero() -> anyhow::Result<()> {
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        assert_eq!(ing.cache_hits.load(Ordering::Relaxed), 0);
        assert_eq!(ing.cache_misses.load(Ordering::Relaxed), 0);
        assert_eq!(ing.cache_stale_hits.load(Ordering::Relaxed), 0);
        Ok(())
    }

    #[test]
    fn freshness_status_cached_when_only_hits_observed() -> anyhow::Result<()> {
        // Status rule: cache present, hits > 0, misses == 0 => Cached.
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        ing.cache_hits.fetch_add(3, Ordering::Relaxed);
        let hits = ing.cache_hits.load(Ordering::Relaxed);
        let misses = ing.cache_misses.load(Ordering::Relaxed);
        let stale_hits = ing.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(ing.cache.is_some(), hits, misses, stale_hits);
        assert!(matches!(status, FreshnessStatus::Cached));
        assert!(reason.is_none());
        Ok(())
    }

    #[test]
    fn freshness_status_fresh_when_any_miss_observed() -> anyhow::Result<()> {
        // Status rule: cache present, any miss => Fresh.
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        ing.cache_hits.fetch_add(2, Ordering::Relaxed);
        ing.cache_misses.fetch_add(1, Ordering::Relaxed);
        let hits = ing.cache_hits.load(Ordering::Relaxed);
        let misses = ing.cache_misses.load(Ordering::Relaxed);
        let stale_hits = ing.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(ing.cache.is_some(), hits, misses, stale_hits);
        assert!(matches!(status, FreshnessStatus::Fresh));
        assert!(reason.is_none());
        Ok(())
    }

    #[test]
    fn freshness_status_stale_when_stale_hit_observed() -> anyhow::Result<()> {
        // Status rule: any stale row used by the adapter makes the
        // source stale, even if other cache lookups hit or miss.
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        ing.cache_hits.fetch_add(2, Ordering::Relaxed);
        ing.cache_misses.fetch_add(1, Ordering::Relaxed);
        ing.cache_stale_hits.fetch_add(1, Ordering::Relaxed);
        let hits = ing.cache_hits.load(Ordering::Relaxed);
        let misses = ing.cache_misses.load(Ordering::Relaxed);
        let stale_hits = ing.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(ing.cache.is_some(), hits, misses, stale_hits);
        assert!(matches!(status, FreshnessStatus::Stale));
        assert_eq!(
            reason.as_deref(),
            Some("one or more expired cache entries were used")
        );
        Ok(())
    }

    #[test]
    fn freshness_status_fresh_when_no_cache_configured() {
        // Status rule: no cache => Fresh regardless of counters (which
        // are unreachable without a cache anyway).
        let ing = make_ingestor("octocat");
        let hits = ing.cache_hits.load(Ordering::Relaxed);
        let misses = ing.cache_misses.load(Ordering::Relaxed);
        let stale_hits = ing.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(ing.cache.is_some(), hits, misses, stale_hits);
        assert!(matches!(status, FreshnessStatus::Fresh));
        assert!(reason.is_none());
    }

    // -- warm-rerun cache behavior --
    //
    // These tests pin the contract that a second `shiplog intake` run
    // against a populated cache reports `cached` freshness. We exercise
    // `fetch_pr_details` directly because (a) it is the cache-aware
    // entry point the rest of the adapter funnels through, and
    // (b) its cache-hit branch returns before any HTTP work — so the
    // test can pre-seed the cache at the canonical key, hand
    // `fetch_pr_details` a constructed `Client` that is never actually
    // used, and observe the counter / status derivation that the
    // intake report's `source_freshness` block reads from. The
    // miss-then-fresh transition is also pinned here at the cache
    // primitive layer (cache.get returns None → set value → cache.get
    // returns Some); the miss branch of `fetch_pr_details` itself
    // requires a working HTTP endpoint (or a recorded-fixtures harness)
    // and is therefore deferred to a follow-up integration fixture.
    //
    // TODO(follow-up): once a recorded-fixtures HTTP harness lands
    // (e.g. `wiremock` or a pre-recorded cassette in `fuzz/` style),
    // extend these to drive a full fresh-then-cached round trip
    // through `ingest()` and assert on `IngestOutput.freshness`.

    fn make_pr_details() -> anyhow::Result<PullRequestDetails> {
        let ts = Utc
            .with_ymd_and_hms(2025, 5, 1, 12, 0, 0)
            .single()
            .ok_or_else(|| anyhow!("Utc.with_ymd_and_hms returned an ambiguous timestamp"))?;
        Ok(PullRequestDetails {
            title: "warm-rerun fixture".into(),
            created_at: ts,
            merged_at: Some(ts),
            additions: 10,
            deletions: 2,
            changed_files: 3,
            base: PullBase {
                repo: PullRepo {
                    full_name: "acme/widgets".into(),
                    html_url: "https://github.com/acme/widgets".into(),
                    private_field: false,
                },
            },
        })
    }

    fn no_op_client() -> Result<Client> {
        // The cache-hit branch of fetch_pr_details returns before
        // touching the client, so the configured user-agent is the
        // only thing we need to make this a well-formed Client.
        // No network call is performed in any of the tests below.
        Client::builder()
            .user_agent("shiplog-warm-rerun-test")
            .build()
            .map_err(Into::into)
    }

    #[test]
    fn warm_rerun_fetch_pr_details_records_cache_hit_without_network() -> anyhow::Result<()> {
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        let url = "https://api.github.com/repos/acme/widgets/pulls/1";

        // Simulate "the first intake run already populated this entry"
        // by writing to the cache directly using the same canonical
        // CacheKey the fetch path constructs.
        let key = CacheKey::pr_details(url);
        let seeded = make_pr_details()?;
        let cache = ing
            .cache
            .as_ref()
            .ok_or_else(|| anyhow!("in-memory cache was just attached"))?;
        cache.set(&key, &seeded)?;

        // First lookup on the SECOND intake run hits the cache. The
        // hit branch in fetch_pr_details returns before calling
        // get_json, so the no-op client is never invoked.
        let client = no_op_client()?;
        let got = ing.fetch_pr_details(&client, url)?;
        assert_eq!(got.title, seeded.title);
        assert_eq!(ing.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(ing.cache_misses.load(Ordering::Relaxed), 0);

        // Second lookup in the same run hits again (a real run can
        // request the same PR twice if details and reviews both refer
        // to it, depending on configuration). Counter must keep
        // climbing; the "no misses" invariant is what makes the status
        // Cached.
        let got_again = ing.fetch_pr_details(&client, url)?;
        assert_eq!(got_again.title, seeded.title);
        assert_eq!(ing.cache_hits.load(Ordering::Relaxed), 2);
        assert_eq!(ing.cache_misses.load(Ordering::Relaxed), 0);

        // Status derivation mirrors the rule baked into
        // GithubIngestor::ingest: cache present, any hit, zero miss
        // => Cached.
        let hits = ing.cache_hits.load(Ordering::Relaxed);
        let misses = ing.cache_misses.load(Ordering::Relaxed);
        let stale_hits = ing.cache_stale_hits.load(Ordering::Relaxed);
        let (status, reason) =
            github_freshness_status(ing.cache.is_some(), hits, misses, stale_hits);
        assert!(
            matches!(status, FreshnessStatus::Cached),
            "warm rerun with fully populated cache must derive Cached, got {status:?}"
        );
        assert!(reason.is_none());
        assert_eq!(status.as_label(), "cached");
        Ok(())
    }

    #[test]
    fn expired_cache_entry_records_stale_hit_without_network() -> anyhow::Result<()> {
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        let url = "https://api.github.com/repos/acme/widgets/pulls/3";
        let key = CacheKey::pr_details(url);
        let seeded = make_pr_details()?;
        let cache = ing
            .cache
            .as_ref()
            .ok_or_else(|| anyhow!("in-memory cache was just attached"))?;
        cache.set_with_ttl(&key, &seeded, chrono::Duration::seconds(-1))?;

        let client = no_op_client()?;
        let got = ing.fetch_pr_details(&client, url)?;
        assert_eq!(got.title, seeded.title);
        assert_eq!(ing.cache_hits.load(Ordering::Relaxed), 1);
        assert_eq!(ing.cache_misses.load(Ordering::Relaxed), 0);
        assert_eq!(ing.cache_stale_hits.load(Ordering::Relaxed), 1);

        let (status, reason) = github_freshness_status(
            ing.cache.is_some(),
            ing.cache_hits.load(Ordering::Relaxed),
            ing.cache_misses.load(Ordering::Relaxed),
            ing.cache_stale_hits.load(Ordering::Relaxed),
        );
        assert!(matches!(status, FreshnessStatus::Stale));
        assert_eq!(
            reason.as_deref(),
            Some("one or more expired cache entries were used")
        );
        assert_eq!(status.as_label(), "stale");
        Ok(())
    }

    struct RecordedGithubServer {
        base_url: String,
        requests: Arc<Mutex<Vec<String>>>,
        handle: Option<JoinHandle<anyhow::Result<()>>>,
    }

    const RECORDED_FIXTURE_READY_TARGET: &str = "/__shiplog_fixture_ready";

    impl RecordedGithubServer {
        fn start(expected_requests: usize) -> anyhow::Result<Self> {
            let listener = TcpListener::bind("127.0.0.1:0").context("bind fixture server")?;
            listener
                .set_nonblocking(true)
                .context("set fixture server nonblocking")?;
            let addr = listener.local_addr()?;
            let base_url = format!("http://{addr}");
            let requests = Arc::new(Mutex::new(Vec::new()));
            let thread_requests = Arc::clone(&requests);
            let thread_base_url = base_url.clone();
            let handle = thread::spawn(move || {
                replay_github_fixtures(
                    listener,
                    &thread_base_url,
                    thread_requests,
                    expected_requests,
                )
            });
            wait_for_recorded_fixture_server(addr)?;

            Ok(Self {
                base_url,
                requests,
                handle: Some(handle),
            })
        }

        fn base_url(&self) -> String {
            self.base_url.clone()
        }

        fn finish(mut self) -> anyhow::Result<Vec<String>> {
            if let Some(handle) = self.handle.take() {
                handle
                    .join()
                    .map_err(|_| anyhow!("recorded fixture server thread panicked"))??;
            }
            self.requests
                .lock()
                .map_err(|_| anyhow!("recorded fixture request log was poisoned"))
                .map(|requests| requests.clone())
        }
    }

    fn replay_github_fixtures(
        listener: TcpListener,
        base_url: &str,
        requests: Arc<Mutex<Vec<String>>>,
        expected_requests: usize,
    ) -> anyhow::Result<()> {
        let deadline = Instant::now() + StdDuration::from_secs(10);

        while fixture_request_count(&requests)? < expected_requests {
            match listener.accept() {
                Ok((mut stream, _peer)) => {
                    stream
                        .set_nonblocking(false)
                        .context("set recorded GitHub fixture stream blocking")?;
                    stream
                        .set_read_timeout(Some(StdDuration::from_secs(5)))
                        .context("set recorded GitHub fixture read timeout")?;
                    stream
                        .set_write_timeout(Some(StdDuration::from_secs(5)))
                        .context("set recorded GitHub fixture write timeout")?;
                    if let Some(request_line) =
                        handle_recorded_github_request(&mut stream, base_url)?
                    {
                        requests
                            .lock()
                            .map_err(|_| anyhow!("recorded fixture request log was poisoned"))?
                            .push(request_line);
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => {
                    if Instant::now() > deadline {
                        return Err(anyhow!(
                            "recorded fixture server expected {expected_requests} requests, saw {}",
                            fixture_request_count(&requests)?
                        ));
                    }
                    thread::sleep(StdDuration::from_millis(10));
                }
                Err(err) => return Err(err).context("accept recorded GitHub fixture request"),
            }
        }

        Ok(())
    }

    fn wait_for_recorded_fixture_server(addr: SocketAddr) -> anyhow::Result<()> {
        let deadline = Instant::now() + StdDuration::from_secs(5);
        loop {
            match TcpStream::connect(addr) {
                Ok(mut stream) => {
                    let request = format!(
                        "GET {RECORDED_FIXTURE_READY_TARGET} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n\r\n"
                    );
                    let probe_result = stream
                        .write_all(request.as_bytes())
                        .and_then(|()| stream.flush())
                        .and_then(|()| {
                            let mut response = Vec::new();
                            stream.read_to_end(&mut response).map(|_| ())
                        });
                    match probe_result {
                        Ok(()) => return Ok(()),
                        Err(err)
                            if matches!(
                                err.kind(),
                                ErrorKind::BrokenPipe
                                    | ErrorKind::ConnectionAborted
                                    | ErrorKind::ConnectionReset
                                    | ErrorKind::Interrupted
                                    | ErrorKind::TimedOut
                                    | ErrorKind::WouldBlock
                            ) =>
                        {
                            if Instant::now() > deadline {
                                return Err(err).context("probe recorded GitHub fixture server");
                            }
                            thread::sleep(StdDuration::from_millis(10));
                        }
                        Err(err) => {
                            return Err(err).context("probe recorded GitHub fixture server");
                        }
                    }
                }
                Err(err)
                    if matches!(
                        err.kind(),
                        ErrorKind::ConnectionRefused
                            | ErrorKind::Interrupted
                            | ErrorKind::TimedOut
                            | ErrorKind::WouldBlock
                    ) =>
                {
                    if Instant::now() > deadline {
                        return Err(err).context("connect recorded GitHub fixture server");
                    }
                    thread::sleep(StdDuration::from_millis(10));
                }
                Err(err) => return Err(err).context("connect recorded GitHub fixture server"),
            }
        }
    }

    fn fixture_request_count(requests: &Arc<Mutex<Vec<String>>>) -> anyhow::Result<usize> {
        requests
            .lock()
            .map_err(|_| anyhow!("recorded fixture request log was poisoned"))
            .map(|requests| requests.len())
    }

    fn handle_recorded_github_request(
        stream: &mut TcpStream,
        base_url: &str,
    ) -> anyhow::Result<Option<String>> {
        let mut buf = [0_u8; 4096];
        let mut received = Vec::new();
        loop {
            let n = match stream.read(&mut buf) {
                Ok(n) => n,
                Err(err)
                    if received.is_empty()
                        && matches!(
                            err.kind(),
                            ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset
                        ) =>
                {
                    return Ok(None);
                }
                Err(err) => return Err(err).context("read recorded GitHub fixture request"),
            };
            if n == 0 {
                if received.is_empty() {
                    return Ok(None);
                }
                break;
            }
            received.extend_from_slice(&buf[..n]);
            if received.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
            if received.len() > 64 * 1024 {
                return Err(anyhow!("recorded GitHub fixture request was too large"));
            }
        }

        let request = String::from_utf8_lossy(&received);
        let request_line = request
            .lines()
            .next()
            .ok_or_else(|| anyhow!("recorded GitHub fixture request had no request line"))?
            .to_string();
        let target = request_line
            .split_whitespace()
            .nth(1)
            .ok_or_else(|| anyhow!("recorded GitHub fixture request had no target"))?;
        if target == RECORDED_FIXTURE_READY_TARGET {
            let response =
                "HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
            stream
                .write_all(response.as_bytes())
                .context("write recorded GitHub fixture readiness response")?;
            stream
                .flush()
                .context("flush recorded GitHub fixture readiness response")?;
            return Ok(None);
        }
        let (status, body) = recorded_github_fixture_response(target, base_url);
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .context("write recorded GitHub fixture response")?;
        stream
            .flush()
            .context("flush recorded GitHub fixture response")?;
        Ok(Some(request_line))
    }

    fn recorded_github_fixture_response(target: &str, base_url: &str) -> (&'static str, String) {
        let body = if target.starts_with("/search/issues?")
            && target_has_query_param(target, "per_page", "1")
        {
            include_str!("../../../tests/fixtures/github-warm-rerun/search_meta.json")
        } else if target.starts_with("/search/issues?")
            && target_has_query_param(target, "per_page", "100")
        {
            include_str!("../../../tests/fixtures/github-warm-rerun/search_items.json")
        } else if target == "/repos/acme/widgets/pulls/1" {
            include_str!("../../../tests/fixtures/github-warm-rerun/pr_details.json")
        } else {
            r#"{"message":"unexpected recorded fixture request"}"#
        };
        let status = if body.contains("unexpected recorded fixture request") {
            "404 Not Found"
        } else {
            "200 OK"
        };
        (status, body.replace("__API_BASE__", base_url))
    }

    fn target_has_query_param(target: &str, key: &str, value: &str) -> bool {
        target
            .split_once('?')
            .map(|(_path, query)| {
                query.split('&').any(|pair| {
                    pair.split_once('=')
                        .map(|(k, v)| k == key && v == value)
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    }

    #[test]
    fn recorded_http_fixtures_prove_full_fresh_then_cached_ingest() -> anyhow::Result<()> {
        let server = RecordedGithubServer::start(5)?;
        let cache_dir = tempfile::tempdir().context("create fixture cache dir")?;

        let mut cold = make_ingestor("octocat").with_cache(cache_dir.path())?;
        cold.api_base = server.base_url();
        let cold_output = cold.ingest()?;
        assert_eq!(cold_output.events.len(), 1);
        let cold_freshness = cold_output
            .freshness
            .first()
            .ok_or_else(|| anyhow!("cold fixture ingest did not emit source freshness"))?;
        assert!(
            matches!(cold_freshness.status, FreshnessStatus::Fresh),
            "first recorded fixture run should be fresh, got {}",
            cold_freshness.status.as_label()
        );
        assert_eq!(cold_freshness.cache_hits, 0);
        assert_eq!(cold_freshness.cache_misses, 1);

        let mut warm = make_ingestor("octocat").with_cache(cache_dir.path())?;
        warm.api_base = server.base_url();
        let warm_output = warm.ingest()?;
        assert_eq!(warm_output.events.len(), 1);
        let warm_freshness = warm_output
            .freshness
            .first()
            .ok_or_else(|| anyhow!("warm fixture ingest did not emit source freshness"))?;
        assert!(
            matches!(warm_freshness.status, FreshnessStatus::Cached),
            "second recorded fixture run should be cached, got {}",
            warm_freshness.status.as_label()
        );
        assert_eq!(warm_freshness.cache_hits, 1);
        assert_eq!(warm_freshness.cache_misses, 0);

        let requests = server.finish()?;
        let search_requests = requests
            .iter()
            .filter(|line| line.contains("/search/issues?"))
            .count();
        let detail_requests = requests
            .iter()
            .filter(|line| line.contains("/repos/acme/widgets/pulls/1"))
            .count();
        assert_eq!(search_requests, 4);
        assert_eq!(
            detail_requests, 1,
            "warm run must serve PR details from cache instead of replaying HTTP"
        );
        Ok(())
    }

    #[test]
    fn warm_rerun_cache_primitive_round_trips_miss_then_hit() -> anyhow::Result<()> {
        // Pin the fresh-vs-cached primitive on the cache itself: a
        // first lookup against an empty cache returns None (so the
        // adapter would increment cache_misses and fetch live), and a
        // subsequent lookup after `set` returns the seeded value (so
        // the adapter would increment cache_hits). This is what backs
        // the warm-rerun status transition; the assertions above prove
        // the adapter increments correctly when fed a hit, and these
        // assertions prove the cache layer itself behaves the way the
        // adapter assumes.
        let ing = make_ingestor("octocat").with_in_memory_cache()?;
        let cache = ing
            .cache
            .as_ref()
            .ok_or_else(|| anyhow!("in-memory cache was just attached"))?;
        let key = CacheKey::pr_details("https://api.github.com/repos/acme/widgets/pulls/2");

        let cold: Option<PullRequestDetails> = cache.get(&key)?;
        assert!(
            cold.is_none(),
            "first lookup against an empty cache must miss"
        );

        let value = make_pr_details()?;
        cache.set(&key, &value)?;
        let warm: Option<PullRequestDetails> = cache.get(&key)?;
        let warm = warm.ok_or_else(|| anyhow!("second lookup after set must hit"))?;
        assert_eq!(warm.title, value.title);
        Ok(())
    }

    // -- api_url --

    #[test]
    fn api_url_concatenates_path() {
        let ing = make_ingestor("octocat");
        assert_eq!(
            ing.api_url("/search/issues"),
            "https://api.github.com/search/issues"
        );
    }

    #[test]
    fn api_url_strips_trailing_slash() {
        let mut ing = make_ingestor("octocat");
        ing.api_base = "https://ghes.local/api/v3/".to_string();
        assert_eq!(
            ing.api_url("/search/issues"),
            "https://ghes.local/api/v3/search/issues"
        );
    }

    // -- html_base_url edge cases --

    #[test]
    fn html_base_url_with_port() {
        let mut ing = make_ingestor("octocat");
        ing.api_base = "https://ghes.local:8443/api/v3".to_string();
        assert_eq!(ing.html_base_url(), "https://ghes.local:8443");
    }

    #[test]
    fn html_base_url_invalid_url_falls_back() {
        let mut ing = make_ingestor("octocat");
        ing.api_base = "not-a-valid-url".to_string();
        assert_eq!(ing.html_base_url(), "https://github.com");
    }

    #[test]
    fn html_base_url_http_scheme() {
        let mut ing = make_ingestor("octocat");
        ing.api_base = "http://internal-ghes.corp/api/v3".to_string();
        assert_eq!(ing.html_base_url(), "http://internal-ghes.corp");
    }

    // -- github_inclusive_range edge cases --

    #[test]
    fn github_inclusive_range_single_day_window() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 3, 16).unwrap(),
        };
        let (start, end) = github_inclusive_range(&window);
        assert_eq!(start, "2025-03-15");
        assert_eq!(end, "2025-03-15");
    }

    #[test]
    fn github_inclusive_range_year_boundary() {
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2024, 12, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        };
        let (start, end) = github_inclusive_range(&window);
        assert_eq!(start, "2024-12-01");
        assert_eq!(end, "2024-12-31");
    }

    #[test]
    fn github_inclusive_range_same_day_uses_pred() {
        // When since == until, pred_opt gives previous day
        let window = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        };
        let (start, end) = github_inclusive_range(&window);
        assert_eq!(start, "2025-06-01");
        assert_eq!(end, "2025-05-31");
    }

    // -- build_url_with_params edge cases --

    #[test]
    fn build_url_with_params_empty_params() {
        let url = build_url_with_params("https://api.github.com/search/issues", &[]).unwrap();
        assert_eq!(url.as_str(), "https://api.github.com/search/issues");
    }

    #[test]
    fn build_url_with_params_special_characters() {
        let url = build_url_with_params(
            "https://api.github.com/search/issues",
            &[(
                "q",
                "author:user+name with spaces&special=chars".to_string(),
            )],
        )
        .unwrap();
        // Should not contain raw spaces
        assert!(!url.as_str().contains(' '));
        // Should roundtrip the value
        let val: String = url
            .query_pairs()
            .find(|(k, _)| k == "q")
            .map(|(_, v)| v.into_owned())
            .unwrap();
        assert_eq!(val, "author:user+name with spaces&special=chars");
    }

    #[test]
    fn build_url_with_params_invalid_base_url_errors() {
        let result = build_url_with_params("not a url", &[]);
        assert!(result.is_err());
    }

    // -- repo_from_repo_url edge cases --

    #[test]
    fn repo_from_repo_url_ghes_url() {
        // GHES API URLs have /api/v3/repos/owner/repo — the function looks for
        // the /repos/ segment, so the path must contain "repos" at position [0].
        // Standard GHES URLs: the path_segments include ["api","v3","repos","owner","repo"].
        // The function only matches when v[0] == "repos", so GHES-style deep paths
        // don't match and fall back to unknown.
        let (full, html) = repo_from_repo_url(
            "https://ghes.corp/api/v3/repos/myorg/myrepo",
            "https://ghes.corp",
        );
        // The function requires path segment [0] == "repos", but GHES has api/v3/repos,
        // so segment[0] == "api". This correctly falls back.
        assert_eq!(full, "unknown/unknown");
        assert_eq!(html, "https://ghes.corp");
    }

    #[test]
    fn repo_from_repo_url_three_plus_segments_wrong_prefix_falls_back() {
        // 3+ segments but v[0] != "repos" → must fall back.
        // Kills && → || mutation: with ||, v.len()>=3 alone would enter the block.
        let (full, html) = repo_from_repo_url(
            "https://api.github.com/users/octocat/repos",
            "https://github.com",
        );
        assert_eq!(full, "unknown/unknown");
        assert_eq!(html, "https://github.com");
    }

    #[test]
    fn repo_from_repo_url_exactly_two_segments_repos_prefix_falls_back() {
        // v[0] == "repos" but only 2 segments → must fall back.
        // Kills && → || mutation: with ||, v[0]=="repos" alone would enter the block.
        let (full, html) = repo_from_repo_url(
            "https://api.github.com/repos/owner-only",
            "https://github.com",
        );
        assert_eq!(full, "unknown/unknown");
        assert_eq!(html, "https://github.com");
    }

    #[test]
    fn repo_from_repo_url_trailing_slash_in_html_base() {
        let (full, html) = repo_from_repo_url(
            "https://api.github.com/repos/owner/repo",
            "https://github.com/",
        );
        assert_eq!(full, "owner/repo");
        assert_eq!(html, "https://github.com/owner/repo");
    }

    #[test]
    fn repo_from_repo_url_extra_path_segments() {
        // URL with more path segments after owner/repo (e.g. /repos/owner/repo/pulls)
        let (full, html) = repo_from_repo_url(
            "https://api.github.com/repos/org/project/pulls",
            "https://github.com",
        );
        assert_eq!(full, "org/project");
        assert_eq!(html, "https://github.com/org/project");
    }

    #[test]
    fn repo_from_repo_url_empty_string() {
        let (full, html) = repo_from_repo_url("", "https://github.com");
        assert_eq!(full, "unknown/unknown");
        assert_eq!(html, "https://github.com");
    }

    // -- build_pr_query date range formatting --

    #[test]
    fn build_pr_query_uses_inclusive_range() {
        let ing = make_ingestor("alice");
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
        };
        let q = ing.build_pr_query(&w);
        // Merged query should use the inclusive end date (2025-03-14)
        assert!(q.contains("2025-03-01..2025-03-14"), "got: {q}");
    }

    #[test]
    fn build_pr_query_unknown_mode_defaults_to_merged() {
        let mut ing = make_ingestor("octocat");
        ing.mode = "unknown_mode".to_string();
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        };
        let q = ing.build_pr_query(&w);
        assert!(
            q.contains("is:merged"),
            "unknown mode should fall through to merged"
        );
    }

    // -- build_reviewed_query format --

    #[test]
    fn build_reviewed_query_uses_updated_qualifier() {
        let ing = make_ingestor("reviewer");
        let w = TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 7, 1).unwrap(),
        };
        let q = ing.build_reviewed_query(&w);
        assert!(
            q.contains("updated:"),
            "review query should use updated: qualifier"
        );
        assert!(q.contains("reviewed-by:reviewer"));
    }

    // -- SearchResponse deserialization --

    #[test]
    fn search_response_deserializes_from_json() {
        let json = r#"{
            "total_count": 42,
            "incomplete_results": false,
            "items": [
                {
                    "id": 1001,
                    "number": 123,
                    "title": "Fix bug",
                    "html_url": "https://github.com/owner/repo/pull/123",
                    "repository_url": "https://api.github.com/repos/owner/repo",
                    "pull_request": { "url": "https://api.github.com/repos/owner/repo/pulls/123" },
                    "created_at": "2025-01-15T10:30:00Z"
                }
            ]
        }"#;

        let resp: SearchResponse<SearchIssueItem> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 42);
        assert!(!resp.incomplete_results);
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].number, 123);
        assert_eq!(resp.items[0].title, "Fix bug");
        assert!(resp.items[0].pull_request.is_some());
    }

    #[test]
    fn search_response_deserializes_without_pull_request() {
        let json = r#"{
            "total_count": 1,
            "incomplete_results": true,
            "items": [
                {
                    "id": 2002,
                    "number": 456,
                    "title": "Issue only",
                    "html_url": "https://github.com/owner/repo/issues/456",
                    "repository_url": "https://api.github.com/repos/owner/repo",
                    "created_at": null
                }
            ]
        }"#;

        let resp: SearchResponse<SearchIssueItem> = serde_json::from_str(json).unwrap();
        assert!(resp.incomplete_results);
        assert!(resp.items[0].pull_request.is_none());
        assert!(resp.items[0].created_at.is_none());
    }

    #[test]
    fn search_response_empty_items() {
        let json = r#"{"total_count": 0, "incomplete_results": false, "items": []}"#;
        let resp: SearchResponse<SearchIssueItem> = serde_json::from_str(json).unwrap();
        assert_eq!(resp.total_count, 0);
        assert!(resp.items.is_empty());
    }

    // -- PullRequestDetails deserialization --

    #[test]
    fn pr_details_deserializes_from_json() {
        let json = r#"{
            "title": "Add feature",
            "created_at": "2025-01-10T08:00:00Z",
            "merged_at": "2025-01-12T14:30:00Z",
            "additions": 150,
            "deletions": 30,
            "changed_files": 5,
            "base": {
                "repo": {
                    "full_name": "owner/repo",
                    "html_url": "https://github.com/owner/repo",
                    "private": false
                }
            }
        }"#;

        let details: PullRequestDetails = serde_json::from_str(json).unwrap();
        assert_eq!(details.title, "Add feature");
        assert!(details.merged_at.is_some());
        assert_eq!(details.additions, 150);
        assert_eq!(details.deletions, 30);
        assert_eq!(details.changed_files, 5);
        assert!(!details.base.repo.private_field);
        assert_eq!(details.base.repo.full_name, "owner/repo");
    }

    #[test]
    fn pr_details_private_repo() {
        let json = r#"{
            "title": "Secret fix",
            "created_at": "2025-01-10T08:00:00Z",
            "merged_at": null,
            "additions": 10,
            "deletions": 5,
            "changed_files": 1,
            "base": {
                "repo": {
                    "full_name": "corp/secret",
                    "html_url": "https://github.com/corp/secret",
                    "private": true
                }
            }
        }"#;

        let details: PullRequestDetails = serde_json::from_str(json).unwrap();
        assert!(details.base.repo.private_field);
        assert!(details.merged_at.is_none());
    }

    // -- PullRequestReview deserialization --

    #[test]
    fn pr_review_deserializes_from_json() {
        let json = r#"{
            "id": 99001,
            "state": "APPROVED",
            "submitted_at": "2025-02-01T12:00:00Z",
            "user": { "login": "reviewer42" }
        }"#;

        let review: PullRequestReview = serde_json::from_str(json).unwrap();
        assert_eq!(review.id, 99001);
        assert_eq!(review.state, "APPROVED");
        assert!(review.submitted_at.is_some());
        assert_eq!(review.user.login, "reviewer42");
    }

    #[test]
    fn pr_review_with_null_submitted_at() {
        let json = r#"{
            "id": 99002,
            "state": "PENDING",
            "submitted_at": null,
            "user": { "login": "pending-reviewer" }
        }"#;

        let review: PullRequestReview = serde_json::from_str(json).unwrap();
        assert!(review.submitted_at.is_none());
    }

    #[test]
    fn recorded_github_search_payload_deserializes_and_converts() {
        let search_payload = serde_json::json!({
            "total_count": 1,
            "incomplete_results": false,
            "items": [
                {
                    "url": "https://api.github.com/repos/octocat/Hello-World/issues/1347",
                    "repository_url": "https://api.github.com/repos/octocat/Hello-World",
                    "labels_url": "https://api.github.com/repos/octocat/Hello-World/issues/1347/labels{/name}",
                    "comments_url": "https://api.github.com/repos/octocat/Hello-World/issues/1347/comments",
                    "events_url": "https://api.github.com/repos/octocat/Hello-World/issues/1347/events",
                    "html_url": "https://github.com/octocat/Hello-World/pull/1347",
                    "id": 1000001347,
                    "node_id": "PR_kwDOABCD",
                    "number": 1347,
                    "state": "closed",
                    "title": "Reduce deploy rollback toil",
                    "user": {
                        "login": "alice",
                        "id": 100,
                        "node_id": "MDQ6VXNlcjEwMA==",
                        "avatar_url": "https://github.com/images/error/alice_happy.gif",
                        "gravatar_id": "",
                        "url": "https://api.github.com/users/alice",
                        "html_url": "https://github.com/alice",
                        "type": "User",
                        "site_admin": false
                    },
                    "labels": [
                        {
                            "id": 208045946,
                            "node_id": "MDU6TGFiZWwyMDgwNDU5NDY=",
                            "url": "https://api.github.com/repos/octocat/Hello-World/labels/reliability",
                            "name": "reliability",
                            "description": "Reliability work",
                            "color": "0e8a16",
                            "default": false
                        }
                    ],
                    "pull_request": {
                        "url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347",
                        "html_url": "https://github.com/octocat/Hello-World/pull/1347",
                        "diff_url": "https://github.com/octocat/Hello-World/pull/1347.diff",
                        "patch_url": "https://github.com/octocat/Hello-World/pull/1347.patch"
                    },
                    "closed_at": "2025-01-18T16:00:00Z",
                    "created_at": "2025-01-10T09:00:00Z",
                    "updated_at": "2025-01-18T16:00:00Z",
                    "author_association": "MEMBER",
                    "score": 1.0
                }
            ]
        });
        let details_payload = serde_json::json!({
            "url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347",
            "id": 2000001347,
            "node_id": "PR_kwDOABCD",
            "html_url": "https://github.com/octocat/Hello-World/pull/1347",
            "diff_url": "https://github.com/octocat/Hello-World/pull/1347.diff",
            "patch_url": "https://github.com/octocat/Hello-World/pull/1347.patch",
            "issue_url": "https://api.github.com/repos/octocat/Hello-World/issues/1347",
            "number": 1347,
            "state": "closed",
            "locked": false,
            "title": "Reduce deploy rollback toil",
            "user": { "login": "alice", "id": 100 },
            "body": "Add preflight checks and rollback runbook links.",
            "created_at": "2025-01-10T09:00:00Z",
            "updated_at": "2025-01-18T16:00:00Z",
            "closed_at": "2025-01-18T16:00:00Z",
            "merged_at": "2025-01-18T16:00:00Z",
            "merge_commit_sha": "ecdd80bb57125d7ba9641ffaa4d7d2c19d3f3091",
            "assignee": null,
            "assignees": [],
            "requested_reviewers": [],
            "requested_teams": [],
            "labels": [],
            "head": {
                "label": "alice:rollback-preflight",
                "ref": "rollback-preflight",
                "sha": "bbcd538c8e72b8c175046e27cc8f907076331401",
                "user": { "login": "alice", "id": 100 },
                "repo": {
                    "id": 1296269,
                    "name": "Hello-World",
                    "full_name": "octocat/Hello-World",
                    "private": false,
                    "html_url": "https://github.com/octocat/Hello-World"
                }
            },
            "base": {
                "label": "octocat:main",
                "ref": "main",
                "sha": "bbcd538c8e72b8c175046e27cc8f907076331402",
                "user": { "login": "octocat", "id": 1 },
                "repo": {
                    "id": 1296269,
                    "name": "Hello-World",
                    "full_name": "octocat/Hello-World",
                    "private": true,
                    "html_url": "https://github.com/octocat/Hello-World"
                }
            },
            "draft": false,
            "merged": true,
            "mergeable": true,
            "rebaseable": true,
            "mergeable_state": "clean",
            "merged_by": { "login": "octocat", "id": 1 },
            "comments": 2,
            "review_comments": 1,
            "commits": 3,
            "additions": 144,
            "deletions": 18,
            "changed_files": 6
        });
        let reviews_payload = serde_json::json!([
            {
                "id": 99001,
                "node_id": "MDE3OlB1bGxSZXF1ZXN0UmV2aWV3OTkwMDE=",
                "user": {
                    "login": "alice",
                    "id": 100,
                    "node_id": "MDQ6VXNlcjEwMA==",
                    "avatar_url": "https://github.com/images/error/alice_happy.gif",
                    "gravatar_id": "",
                    "url": "https://api.github.com/users/alice",
                    "html_url": "https://github.com/alice",
                    "type": "User",
                    "site_admin": false
                },
                "body": "Verified the rollback path.",
                "state": "APPROVED",
                "html_url": "https://github.com/octocat/Hello-World/pull/1347#pullrequestreview-99001",
                "pull_request_url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347",
                "_links": {
                    "html": { "href": "https://github.com/octocat/Hello-World/pull/1347#pullrequestreview-99001" },
                    "pull_request": { "href": "https://api.github.com/repos/octocat/Hello-World/pulls/1347" }
                },
                "submitted_at": "2025-01-18T15:30:00Z",
                "commit_id": "ecdd80bb57125d7ba9641ffaa4d7d2c19d3f3091",
                "author_association": "MEMBER"
            },
            {
                "id": 99002,
                "user": { "login": "bob" },
                "state": "COMMENTED",
                "submitted_at": "2025-01-18T15:45:00Z"
            },
            {
                "id": 99003,
                "user": { "login": "alice" },
                "state": "PENDING",
                "submitted_at": null
            }
        ]);

        let search: SearchResponse<SearchIssueItem> =
            serde_json::from_value(search_payload.clone()).unwrap();
        assert_eq!(search.total_count, 1);
        assert!(!search.incomplete_results);
        assert_eq!(
            search.items[0].repository_url,
            "https://api.github.com/repos/octocat/Hello-World"
        );
        let pr_url = search.items[0].pull_request.as_ref().unwrap().url.clone();

        let details: PullRequestDetails = serde_json::from_value(details_payload).unwrap();
        assert_eq!(details.additions, 144);
        assert_eq!(details.deletions, 18);
        assert_eq!(details.changed_files, 6);
        assert!(details.base.repo.private_field);

        let mut ing = make_ingestor("alice").with_in_memory_cache().unwrap();
        ing.mode = "merged".to_string();
        ing.cache
            .as_ref()
            .unwrap()
            .set(&CacheKey::pr_details(&pr_url), &details)
            .unwrap();

        let client = Client::new();
        let pr_events = ing.items_to_pr_events(&client, search.items).unwrap();
        assert_eq!(pr_events.len(), 1);
        let pr_event = &pr_events[0];
        assert_eq!(pr_event.kind, EventKind::PullRequest);
        assert_eq!(pr_event.actor.login, "alice");
        assert_eq!(pr_event.repo.full_name, "octocat/Hello-World");
        assert_eq!(pr_event.repo.visibility, RepoVisibility::Private);
        assert_eq!(pr_event.source.system, SourceSystem::Github);
        assert_eq!(pr_event.source.url.as_deref(), Some(pr_url.as_str()));
        assert_eq!(pr_event.source.opaque_id.as_deref(), Some("1000001347"));

        if let EventPayload::PullRequest(pr) = &pr_event.payload {
            assert_eq!(pr.number, 1347);
            assert_eq!(pr.title, "Reduce deploy rollback toil");
            assert_eq!(pr.state, PullRequestState::Merged);
            assert_eq!(
                pr.merged_at,
                Some("2025-01-18T16:00:00Z".parse::<DateTime<Utc>>().unwrap())
            );
            assert_eq!(pr.additions, Some(144));
            assert_eq!(pr.deletions, Some(18));
            assert_eq!(pr.changed_files, Some(6));
        } else {
            panic!("expected PullRequest payload");
        }

        let reviews: Vec<PullRequestReview> = serde_json::from_value(reviews_payload).unwrap();
        ing.cache
            .as_ref()
            .unwrap()
            .set(&CacheKey::pr_reviews(&pr_url, 1), &reviews)
            .unwrap();
        let review_search: SearchResponse<SearchIssueItem> =
            serde_json::from_value(search_payload).unwrap();

        let review_events = ing
            .items_to_review_events(&client, review_search.items)
            .unwrap();
        assert_eq!(review_events.len(), 1);
        let review_event = &review_events[0];
        assert_eq!(review_event.kind, EventKind::Review);
        assert_eq!(review_event.actor.login, "alice");
        assert_eq!(review_event.repo.full_name, "octocat/Hello-World");
        assert_eq!(review_event.source.url.as_deref(), Some(pr_url.as_str()));
        assert_eq!(review_event.source.opaque_id.as_deref(), Some("99001"));

        if let EventPayload::Review(review) = &review_event.payload {
            assert_eq!(review.pull_number, 1347);
            assert_eq!(review.pull_title, "Reduce deploy rollback toil");
            assert_eq!(review.state, "APPROVED");
            assert_eq!(
                review.submitted_at,
                "2025-01-18T15:30:00Z".parse::<DateTime<Utc>>().unwrap()
            );
        } else {
            panic!("expected Review payload");
        }
    }

    // -- items_to_pr_events (no network, fetch_details=false) --

    #[test]
    fn items_to_pr_events_without_details_produces_events() {
        let mut ing = make_ingestor("alice");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![
            make_search_item(10, "org/repo-a", true),
            make_search_item(20, "org/repo-b", true),
        ];

        let events = ing.items_to_pr_events(&client, items).unwrap();
        assert_eq!(events.len(), 2);

        assert_eq!(events[0].kind, EventKind::PullRequest);
        assert_eq!(events[0].actor.login, "alice");
        assert_eq!(events[0].repo.full_name, "org/repo-a");
        assert_eq!(events[0].links.len(), 1);
        assert_eq!(events[0].links[0].label, "pr");

        assert_eq!(events[1].repo.full_name, "org/repo-b");
    }

    #[test]
    fn items_to_pr_events_skips_items_without_pr_ref() {
        let mut ing = make_ingestor("bob");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![
            make_search_item(1, "org/repo", true),
            make_search_item(2, "org/repo", false), // no pull_request ref
            make_search_item(3, "org/repo", true),
        ];

        let events = ing.items_to_pr_events(&client, items).unwrap();
        assert_eq!(
            events.len(),
            2,
            "items without pull_request should be skipped"
        );
    }

    #[test]
    fn items_to_pr_events_empty_input() {
        let mut ing = make_ingestor("carol");
        ing.fetch_details = false;
        let client = Client::new();
        let events = ing.items_to_pr_events(&client, vec![]).unwrap();
        assert!(events.is_empty());
    }

    #[test]
    fn items_to_pr_events_sets_source_system() {
        let mut ing = make_ingestor("dave");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![make_search_item(42, "org/repo", true)];
        let events = ing.items_to_pr_events(&client, items).unwrap();

        assert_eq!(events[0].source.system, SourceSystem::Github);
        assert!(events[0].source.url.is_some());
        assert!(events[0].source.opaque_id.is_some());
    }

    #[test]
    fn items_to_pr_events_merged_mode_uses_created_at_as_occurred() {
        let mut ing = make_ingestor("eve");
        ing.fetch_details = false;
        ing.mode = "merged".to_string();

        let client = Client::new();
        let mut item = make_search_item(1, "org/repo", true);
        let created = DateTime::parse_from_rfc3339("2025-03-15T10:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        item.created_at = Some(created);

        let events = ing.items_to_pr_events(&client, vec![item]).unwrap();
        // Without details, merged_at is None, so occurred_at falls back to created_at
        assert_eq!(events[0].occurred_at, created);
    }

    #[test]
    fn items_to_pr_events_created_mode_uses_created_at() {
        let mut ing = make_ingestor("frank");
        ing.fetch_details = false;
        ing.mode = "created".to_string();

        let client = Client::new();
        let mut item = make_search_item(1, "org/repo", true);
        let created = DateTime::parse_from_rfc3339("2025-04-01T12:00:00Z")
            .unwrap()
            .with_timezone(&Utc);
        item.created_at = Some(created);

        let events = ing.items_to_pr_events(&client, vec![item]).unwrap();
        assert_eq!(events[0].occurred_at, created);
    }

    #[test]
    fn items_to_pr_events_without_details_has_unknown_visibility() {
        let mut ing = make_ingestor("grace");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![make_search_item(1, "org/repo", true)];
        let events = ing.items_to_pr_events(&client, items).unwrap();

        assert_eq!(events[0].repo.visibility, RepoVisibility::Unknown);
    }

    #[test]
    fn items_to_pr_events_without_details_state_is_unknown() {
        let mut ing = make_ingestor("heidi");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![make_search_item(1, "org/repo", true)];
        let events = ing.items_to_pr_events(&client, items).unwrap();

        if let EventPayload::PullRequest(ref pr) = events[0].payload {
            assert_eq!(pr.state, PullRequestState::Unknown);
            assert!(pr.merged_at.is_none());
            assert!(pr.additions.is_none());
            assert!(pr.deletions.is_none());
            assert!(pr.changed_files.is_none());
        } else {
            panic!("expected PullRequest payload");
        }
    }

    #[test]
    fn items_to_pr_events_deterministic_ids() {
        let mut ing = make_ingestor("ivan");
        ing.fetch_details = false;

        let client = Client::new();
        let items1 = vec![make_search_item(42, "org/repo", true)];
        let items2 = vec![make_search_item(42, "org/repo", true)];

        let events1 = ing.items_to_pr_events(&client, items1).unwrap();
        let events2 = ing.items_to_pr_events(&client, items2).unwrap();
        assert_eq!(
            events1[0].id, events2[0].id,
            "same inputs should produce same event ID"
        );
    }

    #[test]
    fn items_to_pr_events_different_prs_get_different_ids() {
        let mut ing = make_ingestor("judy");
        ing.fetch_details = false;

        let client = Client::new();
        let items = vec![
            make_search_item(1, "org/repo", true),
            make_search_item(2, "org/repo", true),
        ];

        let events = ing.items_to_pr_events(&client, items).unwrap();
        assert_ne!(events[0].id, events[1].id);
    }

    // -- items_to_review_events (no-network partial) --

    #[test]
    fn items_to_review_events_skips_items_without_pr_ref() {
        let ing = make_ingestor("reviewer");
        let client = Client::new();

        // Item without pull_request ref should be silently skipped.
        // (fetch_pr_reviews would fail, but we never reach it.)
        let items = vec![make_search_item(1, "org/repo", false)];

        let events = ing.items_to_review_events(&client, items).unwrap();
        assert!(events.is_empty());
    }

    // -- ingest error handling --

    #[test]
    fn ingest_rejects_since_equals_until() {
        let date = NaiveDate::from_ymd_opt(2025, 6, 1).unwrap();
        let ing = GithubIngestor::new("user".to_string(), date, date);
        let err = ing.ingest().unwrap_err();
        assert!(
            err.to_string().contains("since must be < until"),
            "got: {err}"
        );
    }

    #[test]
    fn ingest_rejects_since_after_until() {
        let ing = GithubIngestor::new(
            "user".to_string(),
            NaiveDate::from_ymd_opt(2025, 6, 15).unwrap(),
            NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        );
        let err = ing.ingest().unwrap_err();
        assert!(err.to_string().contains("since must be < until"));
    }

    // -- cache integration --

    #[test]
    fn with_cache_then_in_memory_cache_overrides() {
        let temp = tempfile::tempdir().unwrap();
        let ing = make_ingestor("octocat")
            .with_cache(temp.path())
            .unwrap()
            .with_in_memory_cache()
            .unwrap();
        assert!(ing.cache.is_some());
    }

    #[test]
    fn multiple_with_cache_calls_succeed() {
        let temp1 = tempfile::tempdir().unwrap();
        let temp2 = tempfile::tempdir().unwrap();
        let ing = make_ingestor("octocat")
            .with_cache(temp1.path())
            .unwrap()
            .with_cache(temp2.path())
            .unwrap();
        assert!(ing.cache.is_some());
    }

    // ── property tests ──────────────────────────────────────────────────

    fn arb_naive_date() -> impl Strategy<Value = NaiveDate> {
        (2000i32..2030, 1u32..13, 1u32..29)
            .prop_map(|(y, m, d)| NaiveDate::from_ymd_opt(y, m, d).unwrap())
    }

    fn arb_time_window() -> impl Strategy<Value = TimeWindow> {
        (arb_naive_date(), 1u32..366).prop_map(|(since, delta)| {
            let until = since + chrono::Duration::days(delta as i64);
            TimeWindow { since, until }
        })
    }

    proptest! {
        #[test]
        fn prop_github_inclusive_range_start_lte_end(w in arb_time_window()) {
            let (start, end) = github_inclusive_range(&w);
            prop_assert!(start <= end, "start={start} > end={end}");
        }

        #[test]
        fn prop_github_inclusive_range_start_matches_since(w in arb_time_window()) {
            let (start, _) = github_inclusive_range(&w);
            let expected = w.since.format("%Y-%m-%d").to_string();
            prop_assert_eq!(start, expected);
        }

        #[test]
        fn prop_github_inclusive_range_end_is_until_minus_one(w in arb_time_window()) {
            let (_, end) = github_inclusive_range(&w);
            let expected_date = w.until.pred_opt().unwrap_or(w.until);
            let expected = expected_date.format("%Y-%m-%d").to_string();
            prop_assert_eq!(end, expected);
        }

        #[test]
        fn prop_build_url_with_params_produces_valid_url(
            key in "[a-z]{1,10}",
            val in "[a-zA-Z0-9 ]{0,50}",
        ) {
            let result = build_url_with_params(
                "https://api.github.com/search/issues",
                &[(&key, val.clone())],
            );
            prop_assert!(result.is_ok());
            let url = result.unwrap();
            // URL should not contain raw spaces
            prop_assert!(!url.as_str().contains(' '));
            // Value should roundtrip
            let found: String = url.query_pairs()
                .find(|(k, _)| k.as_ref() == key)
                .map(|(_, v)| v.into_owned())
                .unwrap();
            prop_assert_eq!(found, val);
        }

        #[test]
        fn prop_repo_from_repo_url_never_panics(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,19}",
            repo in "[a-zA-Z0-9][a-zA-Z0-9_.-]{0,29}",
        ) {
            let api_url = format!("https://api.github.com/repos/{}/{}", owner, repo);
            let (full, html) = repo_from_repo_url(&api_url, "https://github.com");
            let expected_prefix = format!("{}/", owner);
            prop_assert!(full.starts_with(&expected_prefix));
            prop_assert!(html.starts_with("https://github.com/"));
        }

        #[test]
        fn prop_repo_from_repo_url_arbitrary_strings_never_panic(
            s in ".*",
        ) {
            // Should never panic, even with garbage input
            let _ = repo_from_repo_url(&s, "https://github.com");
        }

        #[test]
        fn prop_build_pr_query_contains_user(
            user in "[a-zA-Z][a-zA-Z0-9-]{0,19}",
        ) {
            let ing = GithubIngestor::new(
                user.clone(),
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            );
            let w = TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            };
            let q = ing.build_pr_query(&w);
            let expected_author = format!("author:{}", user);
            prop_assert!(q.contains(&expected_author));
            prop_assert!(q.contains("is:pr"));
        }

        #[test]
        fn prop_build_reviewed_query_contains_user(
            user in "[a-zA-Z][a-zA-Z0-9-]{0,19}",
        ) {
            let ing = GithubIngestor::new(
                user.clone(),
                NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            );
            let w = TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            };
            let q = ing.build_reviewed_query(&w);
            let expected_reviewer = format!("reviewed-by:{}", user);
            prop_assert!(q.contains(&expected_reviewer));
        }

        #[test]
        fn prop_api_url_preserves_path(
            segment in "[a-z]{1,15}",
        ) {
            let ing = make_ingestor("test");
            let path = format!("/{}", segment);
            let url = ing.api_url(&path);
            prop_assert!(url.ends_with(&path));
            prop_assert!(url.starts_with("https://api.github.com"));
        }

        #[test]
        fn prop_html_base_url_always_returns_valid_string(
            base in "(https?://[a-z]{3,15}\\.[a-z]{2,5}(/[a-z]+)*)",
        ) {
            let mut ing = make_ingestor("test");
            ing.api_base = base;
            let result = ing.html_base_url();
            prop_assert!(!result.is_empty());
            prop_assert!(result.starts_with("http"));
        }
    }
}
