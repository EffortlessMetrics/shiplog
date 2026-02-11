use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, NaiveDate, Utc};
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use shiplog_coverage::{day_windows, month_windows, week_windows, window_len_days};
use shiplog_ids::{EventId, RunId};
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, RepoVisibility, ReviewEvent, SourceRef, SourceSystem,
};
use std::thread::sleep;
use std::time::Duration;
use url::Url;

#[derive(Clone, Debug)]
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
    /// GitHub API base URL (for GHES). Default: https://api.github.com
    pub api_base: String,
}

impl GithubIngestor {
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
        }
    }

    fn client(&self) -> Result<Client> {
        Ok(Client::builder()
            .user_agent("shiplog/0.1")
            .build()
            .context("build reqwest client")?)
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}{}", self.api_base.trim_end_matches('/'), path)
    }

    fn throttle(&self) {
        if self.throttle_ms > 0 {
            sleep(Duration::from_millis(self.throttle_ms));
        }
    }

    fn get_json<T: DeserializeOwned>(&self, client: &Client, url: &str, params: &[(&str, String)]) -> Result<T> {
        let mut req = client.get(url).header("Accept", "application/vnd.github+json");
        req = req.header("X-GitHub-Api-Version", "2022-11-28");
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let req = req.query(params);
        let resp = req.send().with_context(|| format!("GET {url}"))?;
        self.throttle();

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().unwrap_or_default();
            return Err(anyhow!("GitHub API error {status}: {body}"));
        }

        Ok(resp.json::<T>().with_context(|| format!("parse json from {url}"))?)
    }
}

impl Ingestor for GithubIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        if self.since >= self.until {
            return Err(anyhow!("since must be < until"));
        }

        let client = self.client()?;
        let run_id = RunId::now("shiplog");
        let mut slices: Vec<CoverageSlice> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();
        let mut completeness = Completeness::Complete;

        let mut events: Vec<EventEnvelope> = Vec::new();

        // PRs authored
        let pr_query_builder = |w: &TimeWindow| self.build_pr_query(w);
        let (pr_items, pr_slices, pr_partial) = self.collect_search_items(&client, pr_query_builder, self.since, self.until, "prs")?;
        slices.extend(pr_slices);
        if pr_partial {
            completeness = Completeness::Partial;
        }

        events.extend(self.items_to_pr_events(&client, pr_items)?);

        // Reviews authored (best-effort)
        if self.include_reviews {
            warnings.push("Reviews are collected via search + per-PR review fetch; treat as best-effort coverage.".to_string());
            let review_query_builder = |w: &TimeWindow| self.build_reviewed_query(w);
            let (review_items, review_slices, review_partial) = self.collect_search_items(&client, review_query_builder, self.since, self.until, "reviews")?;
            slices.extend(review_slices);
            if review_partial {
                completeness = Completeness::Partial;
            }
            events.extend(self.items_to_review_events(&client, review_items)?);
        }

        // Sort for stable output
        events.sort_by_key(|e| e.occurred_at);

        let cov = CoverageManifest {
            run_id,
            generated_at: Utc::now(),
            user: self.user.clone(),
            window: TimeWindow { since: self.since, until: self.until },
            mode: self.mode.clone(),
            sources: vec!["github".to_string()],
            slices,
            warnings,
            completeness,
        };

        Ok(IngestOutput { events, coverage: cov })
    }
}

impl GithubIngestor {
    fn build_pr_query(&self, w: &TimeWindow) -> String {
        let (start, end) = github_inclusive_range(w);
        match self.mode.as_str() {
            "created" => format!("is:pr author:{} created:{}..{}", self.user, start, end),
            _ => format!("is:pr is:merged author:{} merged:{}..{}", self.user, start, end),
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
            let (mut i, mut s, p) = self.collect_window(client, &make_query, &w, Granularity::Month, label)?;
            items.append(&mut i);
            slices.append(&mut s);
            partial |= p;
        }

        Ok((items, slices, partial))
    }

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
                if meta_total > 1000 { "cap" } else { "incomplete" }
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
                let (mut i, mut s, p) = self.collect_window(client, make_query, &sub, gran.next(), label)?;
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
            slices[0].notes.push("partial:unresolvable_at_this_granularity".to_string());
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

    fn items_to_pr_events(&self, client: &Client, items: Vec<SearchIssueItem>) -> Result<Vec<EventEnvelope>> {
        let mut out = Vec::new();
        for item in items {
            if let Some(pr_ref) = &item.pull_request {
                let (repo_full_name, repo_html_url) = repo_from_repo_url(&item.repository_url);

                let (title, created_at, merged_at, additions, deletions, changed_files, visibility) =
                    if self.fetch_details {
                        match self.fetch_pr_details(client, &pr_ref.url) {
                            Ok(d) => {
                                let vis = if d.base.repo.private_field { RepoVisibility::Private } else { RepoVisibility::Public };
                                (
                                    d.title,
                                    d.created_at,
                                    d.merged_at,
                                    Some(d.additions),
                                    Some(d.deletions),
                                    Some(d.changed_files),
                                    vis,
                                )
                            }
                            Err(_) => {
                                // If details fail, fall back to search fields.
                                (
                                    item.title.clone(),
                                    item.created_at.unwrap_or_else(Utc::now),
                                    None,
                                    None,
                                    None,
                                    None,
                                    RepoVisibility::Unknown,
                                )
                            }
                        }
                    } else {
                        (
                            item.title.clone(),
                            item.created_at.unwrap_or_else(Utc::now),
                            None,
                            None,
                            None,
                            None,
                            RepoVisibility::Unknown,
                        )
                    };

                let occurred_at = match self.mode.as_str() {
                    "created" => created_at,
                    _ => merged_at.unwrap_or(created_at),
                };

                let state = if merged_at.is_some() {
                    PullRequestState::Merged
                } else {
                    PullRequestState::Unknown
                };

                let id = EventId::from_parts([
                    "github",
                    "pr",
                    &repo_full_name,
                    &item.number.to_string(),
                ]);

                let ev = EventEnvelope {
                    id,
                    kind: EventKind::PullRequest,
                    occurred_at,
                    actor: Actor { login: self.user.clone(), id: None },
                    repo: RepoRef {
                        full_name: repo_full_name,
                        html_url: Some(repo_html_url),
                        visibility,
                    },
                    payload: EventPayload::PullRequest(PullRequestEvent {
                        number: item.number,
                        title,
                        state,
                        created_at,
                        merged_at,
                        additions,
                        deletions,
                        changed_files,
                        touched_paths_hint: vec![],
                        window: None,
                    }),
                    tags: vec![],
                    links: vec![Link { label: "pr".into(), url: item.html_url.clone() }],
                    source: SourceRef { system: SourceSystem::Github, url: Some(pr_ref.url.clone()), opaque_id: Some(item.id.to_string()) },
                };

                out.push(ev);
            }
        }
        Ok(out)
    }

    fn items_to_review_events(&self, client: &Client, items: Vec<SearchIssueItem>) -> Result<Vec<EventEnvelope>> {
        let mut out = Vec::new();
        for item in items {
            let Some(pr_ref) = &item.pull_request else { continue };
            let (repo_full_name, repo_html_url) = repo_from_repo_url(&item.repository_url);

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
                    actor: Actor { login: self.user.clone(), id: None },
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
                    links: vec![Link { label: "pr".into(), url: item.html_url.clone() }],
                    source: SourceRef { system: SourceSystem::Github, url: Some(pr_ref.url.clone()), opaque_id: Some(r.id.to_string()) },
                };

                out.push(ev);
            }
        }
        Ok(out)
    }

    fn fetch_pr_details(&self, client: &Client, pr_api_url: &str) -> Result<PullRequestDetails> {
        self.get_json(client, pr_api_url, &[])
    }

    fn fetch_pr_reviews(&self, client: &Client, pr_api_url: &str) -> Result<Vec<PullRequestReview>> {
        let url = format!("{pr_api_url}/reviews");
        let mut out = Vec::new();
        let per_page = 100;
        for page in 1..=10 {
            let page_reviews: Vec<PullRequestReview> = self.get_json(
                client,
                &url,
                &[
                    ("per_page", per_page.to_string()),
                    ("page", page.to_string()),
                ],
            )?;
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

fn repo_from_repo_url(repo_api_url: &str) -> (String, String) {
    if let Ok(u) = Url::parse(repo_api_url) {
        if let Some(segs) = u.path_segments() {
            let v: Vec<&str> = segs.collect();
            if v.len() >= 3 && v[0] == "repos" {
                let owner = v[1];
                let repo = v[2];
                let full = format!("{}/{}", owner, repo);
                let html = format!("https://github.com/{}/{}", owner, repo);
                return (full, html);
            }
        }
    }
    ("unknown/unknown".to_string(), "https://github.com".to_string())
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

#[derive(Debug, Deserialize)]
struct PullRequestDetails {
    title: String,
    created_at: DateTime<Utc>,
    merged_at: Option<DateTime<Utc>>,
    additions: u64,
    deletions: u64,
    changed_files: u64,
    base: PullBase,
}

#[derive(Debug, Deserialize)]
struct PullBase {
    repo: PullRepo,
}

#[derive(Debug, Deserialize)]
struct PullRepo {
    full_name: String,
    html_url: String,
    #[serde(rename = "private")]
    private_field: bool,
}

#[derive(Debug, Deserialize)]
struct PullRequestReview {
    id: u64,
    state: String,
    submitted_at: Option<DateTime<Utc>>,
    user: ReviewUser,
}

#[derive(Debug, Deserialize)]
struct ReviewUser {
    login: String,
}
