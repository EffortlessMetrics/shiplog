use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use shiplog::coverage::month_windows;
use shiplog::schema::coverage::TimeWindow;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::*;

const STATIC_SEARCH_REQUESTS_PER_QUERY: u64 = 11;
const STATIC_MAX_RESULTS_PER_QUERY: u64 = 1000;

pub(super) fn run_plan(args: GithubActivityPlanArgs) -> Result<()> {
    let config = load_config_for_command(&args.config)?;
    ensure_supported_config_version(&config)?;

    let base_dir = config_base_dir(&args.config);
    let out_dir = args
        .out
        .as_deref()
        .map(|path| resolve_config_path(Path::new("."), path))
        .unwrap_or_else(|| github_activity_default_out(&config, &base_dir));
    let plan = build_plan(&config, args.profile)?;

    std::fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let plan_path = out_dir.join(GITHUB_ACTIVITY_PLAN_FILENAME);
    let json = serde_json::to_string_pretty(&plan).context("serialize GitHub activity plan")?;
    ensure_no_secret_sentinels(GITHUB_ACTIVITY_PLAN_FILENAME, &json)?;
    std::fs::write(&plan_path, format!("{json}\n"))
        .with_context(|| format!("write {}", plan_path.display()))?;

    println!("GitHub activity plan written:");
    println!("- {}", display_path_for_cli(&plan_path));
    println!("Actor: {}", plan.actor);
    if plan.repo_owners.is_empty() {
        println!("Repository owners: actor-wide (no owner filter requested)");
    } else {
        println!("Repository owners: {}", plan.repo_owners.join(", "));
    }
    println!("Profile: {}", plan.profile);
    println!("Windows: {}", plan.windows.len());
    println!(
        "Estimated requests: search {}, core {}, review {}",
        plan.estimated_totals.search_requests,
        plan.estimated_totals.core_requests,
        plan.estimated_totals.review_requests
    );
    println!("Provider calls: none (static plan)");
    println!("Writes: {}", display_path_for_cli(&plan_path));
    println!(
        "Next: scout/run are not implemented yet; review the plan receipt before spending API."
    );

    Ok(())
}

fn build_plan(
    config: &ShiplogConfig,
    profile_override: Option<GithubActivityProfile>,
) -> Result<GithubActivityPlanReceipt> {
    let activity = &config.github_activity;
    if activity.include_comments {
        anyhow::bail!("github_activity.include_comments is not supported by activity planning yet");
    }
    if activity.include_commits {
        anyhow::bail!("github_activity.include_commits is not supported by activity planning yet");
    }

    let actor = github_activity_actor(config)?;
    let repo_owners = normalized_owner_list(&activity.repo_owners);
    let since = activity
        .since
        .ok_or_else(|| anyhow::anyhow!("github_activity.since is required"))?;
    let until = activity
        .until
        .ok_or_else(|| anyhow::anyhow!("github_activity.until is required"))?;
    if since >= until {
        anyhow::bail!("github_activity must satisfy since < until");
    }

    let profile = match profile_override {
        Some(profile) => profile,
        None => parse_activity_profile(activity.profile.as_deref())?
            .unwrap_or(GithubActivityProfile::Scout),
    };
    let include_authored = activity.include_authored_prs.unwrap_or(true);
    let include_reviews = match profile {
        GithubActivityProfile::Full => activity.include_reviews.unwrap_or(true),
        GithubActivityProfile::Scout | GithubActivityProfile::Authored => false,
    };
    if !include_authored && !include_reviews {
        anyhow::bail!("GitHub activity plan has no query kinds; enable authored PRs or reviews");
    }

    let mode = github_activity_mode(config)?;
    let budget_policy = budget_policy(&activity.budget)?;
    let mut windows = Vec::new();
    let mut totals = GithubActivityEstimatedTotals::default();
    for window in month_windows(since, until) {
        let mut queries = Vec::new();
        if include_authored {
            queries.push(plan_query(
                "authored_prs",
                &build_authored_query(&actor, &mode, &window),
                profile,
            ));
        }
        if include_reviews {
            queries.push(plan_query(
                "reviewed_prs",
                &build_reviewed_query(&actor, &window),
                profile,
            ));
        }
        for query in &queries {
            totals.search_requests += query.estimated_search_requests;
            totals.core_requests += query.estimated_core_requests + query.estimated_review_requests;
            totals.review_requests += query.estimated_review_requests;
        }
        windows.push(GithubActivityPlanWindow {
            window_id: window_id(&window),
            since: window.since.to_string(),
            until: window.until.to_string(),
            granularity: "month".to_string(),
            query_kinds: queries
                .iter()
                .map(|query| query.query_kind.clone())
                .collect(),
            queries,
        });
    }

    Ok(GithubActivityPlanReceipt {
        schema_version: GITHUB_ACTIVITY_PLAN_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        shiplog_version: env!("CARGO_PKG_VERSION").to_string(),
        activity_id: activity_id(&actor, since, until, profile, &repo_owners),
        actor,
        repo_owners,
        owner_filter_requested: !activity.repo_owners.is_empty(),
        query_strategy: "actor_search_owner_filter".to_string(),
        profile: profile.as_str().to_string(),
        since: since.to_string(),
        until: until.to_string(),
        planning_mode: "static".to_string(),
        windows,
        estimated_totals: totals,
        budget_policy,
        next_actions: Vec::new(),
        receipt_refs: Vec::new(),
    })
}

fn github_activity_default_out(config: &ShiplogConfig, base_dir: &Path) -> PathBuf {
    if let Some(cache_dir) = config.github_activity.cache_dir.as_ref() {
        let cache_dir = resolve_config_path(base_dir, cache_dir);
        if cache_dir
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name == ".cache")
            && let Some(parent) = cache_dir.parent()
        {
            return parent.to_path_buf();
        }
    }
    config_default_out(config, base_dir)
}

fn github_activity_actor(config: &ShiplogConfig) -> Result<String> {
    if let Some(actor) = non_empty_string(config.github_activity.actor.as_deref()) {
        return Ok(actor);
    }
    if let Some(actor) = config
        .sources
        .github
        .as_ref()
        .and_then(|source| non_empty_string(source.user.as_deref()))
    {
        return Ok(actor);
    }
    if config
        .sources
        .github
        .as_ref()
        .is_some_and(|source| source.me)
    {
        anyhow::bail!(
            "github_activity.actor is required for static planning; sources.github.me requires identity discovery"
        );
    }
    anyhow::bail!(
        "github_activity.actor is required; sources.github.user is accepted as a compatibility alias"
    )
}

fn github_activity_mode(config: &ShiplogConfig) -> Result<String> {
    let mode = config
        .sources
        .github
        .as_ref()
        .and_then(|source| non_empty_string(source.mode.as_deref()))
        .unwrap_or_else(|| "created".to_string());
    match mode.as_str() {
        "created" | "merged" => Ok(mode),
        _ => anyhow::bail!("sources.github.mode must be merged or created, got {mode:?}"),
    }
}

fn parse_activity_profile(value: Option<&str>) -> Result<Option<GithubActivityProfile>> {
    let Some(value) = non_empty_string(value) else {
        return Ok(None);
    };
    match value.as_str() {
        "scout" => Ok(GithubActivityProfile::Scout),
        "authored" => Ok(GithubActivityProfile::Authored),
        "full" => Ok(GithubActivityProfile::Full),
        _ => {
            anyhow::bail!("github_activity.profile must be scout, authored, or full, got {value:?}")
        }
    }
    .map(Some)
}

fn normalized_owner_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| non_empty_string(Some(value)))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn budget_policy(config: &ConfigGithubActivityBudget) -> Result<GithubActivityBudgetPolicy> {
    let on_exhausted = non_empty_string(config.on_exhausted.as_deref())
        .unwrap_or_else(|| "checkpoint_and_stop".to_string());
    if on_exhausted != "checkpoint_and_stop" {
        anyhow::bail!(
            "github_activity.budget.on_exhausted must be checkpoint_and_stop, got {on_exhausted:?}"
        );
    }
    Ok(GithubActivityBudgetPolicy {
        max_search_requests: config.max_search_requests.unwrap_or(300),
        max_core_requests: config.max_core_requests.unwrap_or(1000),
        max_search_per_minute: config.max_search_per_minute.unwrap_or(24),
        on_exhausted,
    })
}

fn plan_query(
    query_kind: &str,
    search_query: &str,
    profile: GithubActivityProfile,
) -> GithubActivityPlanQuery {
    let estimated_detail_requests = match (query_kind, profile) {
        ("authored_prs", GithubActivityProfile::Authored | GithubActivityProfile::Full) => {
            STATIC_MAX_RESULTS_PER_QUERY
        }
        _ => 0,
    };
    let estimated_review_requests = match (query_kind, profile) {
        ("reviewed_prs", GithubActivityProfile::Full) => STATIC_MAX_RESULTS_PER_QUERY,
        _ => 0,
    };
    GithubActivityPlanQuery {
        query_kind: query_kind.to_string(),
        search_query: search_query.to_string(),
        estimated_search_requests: STATIC_SEARCH_REQUESTS_PER_QUERY,
        estimated_core_requests: estimated_detail_requests,
        estimated_review_requests,
        dense_window_risk: "unknown".to_string(),
        cache_reuse: "unknown".to_string(),
    }
}

fn build_authored_query(actor: &str, mode: &str, window: &TimeWindow) -> String {
    let (start, end) = inclusive_range(window);
    match mode {
        "merged" => format!("is:pr is:merged author:{actor} merged:{start}..{end}"),
        _ => format!("is:pr author:{actor} created:{start}..{end}"),
    }
}

fn build_reviewed_query(actor: &str, window: &TimeWindow) -> String {
    let (start, end) = inclusive_range(window);
    format!("is:pr reviewed-by:{actor} updated:{start}..{end}")
}

fn inclusive_range(window: &TimeWindow) -> (NaiveDate, NaiveDate) {
    (window.since, window.until - Duration::days(1))
}

fn window_id(window: &TimeWindow) -> String {
    window.since.format("%Y-%m").to_string()
}

fn activity_id(
    actor: &str,
    since: NaiveDate,
    until: NaiveDate,
    profile: GithubActivityProfile,
    repo_owners: &[String],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(actor.as_bytes());
    hasher.update(b"\n");
    hasher.update(since.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(until.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(profile.as_str().as_bytes());
    for owner in repo_owners {
        hasher.update(b"\n");
        hasher.update(owner.as_bytes());
    }
    let hex = hex::encode(hasher.finalize());
    format!("github_activity_{}", &hex[..12])
}

#[derive(Serialize)]
struct GithubActivityPlanReceipt {
    schema_version: String,
    generated_at: String,
    shiplog_version: String,
    activity_id: String,
    actor: String,
    repo_owners: Vec<String>,
    owner_filter_requested: bool,
    query_strategy: String,
    profile: String,
    since: String,
    until: String,
    planning_mode: String,
    windows: Vec<GithubActivityPlanWindow>,
    estimated_totals: GithubActivityEstimatedTotals,
    budget_policy: GithubActivityBudgetPolicy,
    next_actions: Vec<GithubActivityNextAction>,
    receipt_refs: Vec<String>,
}

#[derive(Serialize)]
struct GithubActivityPlanWindow {
    window_id: String,
    since: String,
    until: String,
    granularity: String,
    query_kinds: Vec<String>,
    queries: Vec<GithubActivityPlanQuery>,
}

#[derive(Serialize)]
struct GithubActivityPlanQuery {
    query_kind: String,
    search_query: String,
    estimated_search_requests: u64,
    estimated_core_requests: u64,
    estimated_review_requests: u64,
    dense_window_risk: String,
    cache_reuse: String,
}

#[derive(Default, Serialize)]
struct GithubActivityEstimatedTotals {
    search_requests: u64,
    core_requests: u64,
    review_requests: u64,
}

#[derive(Serialize)]
struct GithubActivityBudgetPolicy {
    max_search_requests: u64,
    max_core_requests: u64,
    max_search_per_minute: u64,
    on_exhausted: String,
}

#[derive(Serialize)]
struct GithubActivityNextAction {
    command: String,
    writes: bool,
    reason: String,
}
