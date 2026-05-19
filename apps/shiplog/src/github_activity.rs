use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shiplog::coverage::month_windows;
use shiplog::ingest::github::{
    GithubApiBudget, GithubApiCacheCounts, GithubApiRequestCounts, GithubRateLimitSnapshot,
    GithubSecondaryLimitEvent,
};
use shiplog::schema::coverage::{CoverageManifest, TimeWindow};
use std::collections::{BTreeMap, BTreeSet};
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
    for action in &plan.next_actions {
        println!(
            "Next: {} [{}] - {}",
            action.command,
            write_posture(action.writes),
            action.reason
        );
    }

    Ok(())
}

pub(super) fn run_scout(args: GithubActivityRunArgs) -> Result<()> {
    if args
        .profile
        .is_some_and(|profile| profile != GithubActivityProfile::Scout)
    {
        anyhow::bail!(
            "github activity scout always uses the scout profile; use `shiplog github activity run --profile {}` instead",
            args.profile
                .map(GithubActivityProfile::as_str)
                .unwrap_or("scout")
        );
    }
    run_activity_profile(args, Some(GithubActivityProfile::Scout))
}

pub(super) fn run_activity(args: GithubActivityRunArgs) -> Result<()> {
    let profile = args.profile;
    run_activity_profile(args, profile)
}

fn run_activity_profile(
    args: GithubActivityRunArgs,
    profile_override: Option<GithubActivityProfile>,
) -> Result<()> {
    let config = load_config_for_command(&args.config)?;
    ensure_supported_config_version(&config)?;

    let base_dir = config_base_dir(&args.config);
    let out_dir = args
        .out
        .as_deref()
        .map(|path| resolve_config_path(Path::new("."), path))
        .unwrap_or_else(|| github_activity_default_out(&config, &base_dir));
    std::fs::create_dir_all(&out_dir).with_context(|| format!("create {}", out_dir.display()))?;

    let plan = build_plan(&config, profile_override)?;
    let plan_path = out_dir.join(GITHUB_ACTIVITY_PLAN_FILENAME);
    write_json_receipt(&plan_path, GITHUB_ACTIVITY_PLAN_FILENAME, &plan)?;

    let progress_path = out_dir.join(GITHUB_ACTIVITY_PROGRESS_FILENAME);
    let api_ledger_path = out_dir.join(GITHUB_ACTIVITY_API_LEDGER_FILENAME);
    if args.resume
        && let Some(progress) = load_progress_if_completed(&progress_path, &plan)?
    {
        if api_ledger_path.exists() {
            println!("GitHub activity {} already completed.", progress.profile);
            println!("- {}", display_path_for_cli(&progress_path));
            println!("- {}", display_path_for_cli(&api_ledger_path));
            println!("Provider calls: none (--resume)");
            return Ok(());
        }
        println!(
            "GitHub activity {} progress is completed but API ledger is missing; rerunning to refresh receipts.",
            progress.profile
        );
    }

    let execution = activity_execution(&config, &base_dir, &out_dir, plan.profile_enum()?)?;
    let start_state = match plan.profile_enum()? {
        GithubActivityProfile::Scout => "scouting",
        GithubActivityProfile::Authored | GithubActivityProfile::Full => "running",
    };
    let progress = progress_receipt(
        &plan,
        start_state,
        Vec::new(),
        plan.window_ids(),
        None,
        None,
        vec![GITHUB_ACTIVITY_PLAN_FILENAME.to_string()],
    );
    write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;

    let ing = make_github_ingestor(
        &plan.actor,
        execution.since,
        execution.until,
        &execution.mode,
        plan.repo_owners.clone(),
        execution.include_reviews,
        execution.no_details,
        execution.throttle_ms,
        None,
        &execution.api_base,
        execution.cache_dir,
    )
    .context("create GitHub activity ingestor")?
    .with_api_budget(GithubApiBudget {
        max_search_requests: Some(plan.budget_policy.max_search_requests),
        max_core_requests: Some(plan.budget_policy.max_core_requests),
    });

    let ingest = match ing.ingest() {
        Ok(ingest) => ingest,
        Err(err) => {
            let counts = ing.api_request_counts();
            let cache_counts = ing.api_cache_counts();
            let rate_limit_snapshots = ing.rate_limit_snapshots();
            let secondary_limit_events = ing.secondary_limit_events();
            let mut progress = progress_receipt(
                &plan,
                "checkpointed",
                Vec::new(),
                plan.window_ids(),
                None,
                Some(activity_stop_reason(&err)),
                vec![
                    GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
                    GITHUB_ACTIVITY_API_LEDGER_FILENAME.to_string(),
                ],
            );
            progress.budget_checkpoint = Some(GithubActivityProgressBudget {
                search_requests: counts.search,
                core_requests: counts.core,
            });
            write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;
            let api_ledger = api_ledger_receipt(
                &plan,
                counts,
                cache_counts,
                rate_limit_snapshots,
                secondary_limit_events,
                Some(activity_stop_reason(&err)),
                OwnerFilterLedger::from_plan(&plan),
            );
            write_json_receipt(
                &api_ledger_path,
                GITHUB_ACTIVITY_API_LEDGER_FILENAME,
                &api_ledger,
            )?;
            println!(
                "GitHub activity {} checkpointed after search {}, core {} request(s).",
                plan.profile, counts.search, counts.core
            );
            println!("- {}", display_path_for_cli(&progress_path));
            println!("- {}", display_path_for_cli(&api_ledger_path));
            return Err(err).context("GitHub activity ingest stopped before completion");
        }
    };

    let api_counts = ing.api_request_counts();
    let cache_counts = ing.api_cache_counts();
    let rate_limit_snapshots = ing.rate_limit_snapshots();
    let secondary_limit_events = ing.secondary_limit_events();
    let owner_filter = OwnerFilterLedger::from_coverage(&plan, &ingest.coverage);
    let api_ledger = api_ledger_receipt(
        &plan,
        api_counts,
        cache_counts,
        rate_limit_snapshots,
        secondary_limit_events,
        None,
        owner_filter,
    );
    write_json_receipt(
        &api_ledger_path,
        GITHUB_ACTIVITY_API_LEDGER_FILENAME,
        &api_ledger,
    )?;

    let run_id = ingest.coverage.run_id.to_string();
    let run_dir = out_dir.join(&run_id);
    let bundle_profile = BundleProfile::Internal;
    let redaction_key = RedactionKey::resolve(None, &bundle_profile)?;
    let clusterer = build_clusterer(false, "", "", None);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());
    let cache_path = DeterministicRedactor::cache_path(&run_dir);
    let _ = redactor.load_cache(&cache_path);
    let window_label = format!("github activity {}", plan.profile);
    let (outputs, ws_source) = engine
        .run(
            ingest,
            &plan.actor,
            &window_label,
            &run_dir,
            false,
            &bundle_profile,
        )
        .context("run GitHub activity pipeline")?;
    redactor
        .save_cache(&cache_path)
        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

    let run_ref = run_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| run_id.clone());
    let progress = progress_receipt(
        &plan,
        "completed",
        plan.window_ids(),
        Vec::new(),
        Some(run_ref.as_str()),
        None,
        vec![
            GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
            GITHUB_ACTIVITY_API_LEDGER_FILENAME.to_string(),
            format!("{run_ref}/intake.report.json"),
            format!("{run_ref}/coverage.manifest.json"),
        ],
    );
    write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;

    println!("GitHub activity {} completed.", plan.profile);
    println!("Receipts:");
    println!("- {}", display_path_for_cli(&plan_path));
    println!("- {}", display_path_for_cli(&progress_path));
    println!("- {}", display_path_for_cli(&api_ledger_path));
    println!("Run artifacts:");
    print_outputs(&outputs, ws_source);
    for action in activity_next_actions(&plan, &out_dir) {
        println!("Next: {action}");
    }

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
    let repo_owners = github_activity_repo_owners(config);
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
        owner_filter_requested: !repo_owners.is_empty(),
        repo_owners,
        query_strategy: "actor_search_owner_filter".to_string(),
        profile: profile.as_str().to_string(),
        since: since.to_string(),
        until: until.to_string(),
        planning_mode: "static".to_string(),
        windows,
        estimated_totals: totals,
        budget_policy,
        next_actions: activity_plan_next_actions(profile),
        receipt_refs: Vec::new(),
    })
}

fn activity_execution(
    config: &ShiplogConfig,
    base_dir: &Path,
    out_dir: &Path,
    profile: GithubActivityProfile,
) -> Result<GithubActivityExecution> {
    if config.github_activity.include_authored_prs == Some(false) {
        anyhow::bail!(
            "github activity run currently requires include_authored_prs=true; plan-only may be used for review-only scope"
        );
    }

    let since = config
        .github_activity
        .since
        .ok_or_else(|| anyhow::anyhow!("github_activity.since is required"))?;
    let until = config
        .github_activity
        .until
        .ok_or_else(|| anyhow::anyhow!("github_activity.until is required"))?;
    let source = config.sources.github.as_ref();
    let no_cache = source.is_some_and(|source| source.no_cache);
    let api_base = source
        .and_then(|source| non_empty_string(source.api_base.as_deref()))
        .unwrap_or_else(|| "https://api.github.com".to_string());
    let mode = github_activity_mode(config)?;
    let budget = budget_policy(&config.github_activity.budget)?;
    let budget_throttle_ms = throttle_for_search_budget(budget.max_search_per_minute)?;
    let source_throttle_ms = source.map(|source| source.throttle_ms).unwrap_or_default();
    let cache_dir = if no_cache {
        None
    } else if let Some(cache_dir) = config.github_activity.cache_dir.as_ref() {
        Some(resolve_config_path(base_dir, cache_dir))
    } else {
        source
            .and_then(|source| source.cache_dir.as_ref())
            .map(|cache_dir| resolve_config_path(base_dir, cache_dir))
            .or_else(|| Some(out_dir.join(".cache")))
    };
    let (include_reviews, no_details) = match profile {
        GithubActivityProfile::Scout => (false, true),
        GithubActivityProfile::Authored => (false, false),
        GithubActivityProfile::Full => (
            config.github_activity.include_reviews.unwrap_or(true),
            false,
        ),
    };

    Ok(GithubActivityExecution {
        since,
        until,
        mode,
        include_reviews,
        no_details,
        throttle_ms: source_throttle_ms.max(budget_throttle_ms),
        api_base,
        cache_dir,
    })
}

fn throttle_for_search_budget(max_search_per_minute: u64) -> Result<u64> {
    if max_search_per_minute == 0 {
        anyhow::bail!("github_activity.budget.max_search_per_minute must be greater than zero");
    }
    Ok(60_000_u64.div_ceil(max_search_per_minute))
}

fn activity_plan_next_actions(profile: GithubActivityProfile) -> Vec<GithubActivityNextAction> {
    let (command, reason) = match profile {
        GithubActivityProfile::Scout => (
            "shiplog github activity scout --resume",
            "Run the search-only scout before fetching details.",
        ),
        GithubActivityProfile::Authored => (
            "shiplog github activity run --profile authored --resume",
            "Fetch authored PR details using the planned activity scope.",
        ),
        GithubActivityProfile::Full => (
            "shiplog github activity run --profile full --resume",
            "Fetch authored PR details and review activity using the planned activity scope.",
        ),
    };
    vec![GithubActivityNextAction {
        command: command.to_string(),
        writes: true,
        reason: reason.to_string(),
    }]
}

fn activity_next_actions(plan: &GithubActivityPlanReceipt, out_dir: &Path) -> Vec<String> {
    let out = quote_cli_value(&display_path_for_cli(out_dir));
    match plan.profile_enum() {
        Ok(GithubActivityProfile::Scout) => vec![format!(
            "shiplog github activity run --out {out} --profile authored --resume [writes]"
        )],
        Ok(GithubActivityProfile::Authored) => vec![format!(
            "shiplog github activity run --out {out} --profile full --resume [writes]"
        )],
        Ok(GithubActivityProfile::Full) => {
            vec![format!("shiplog status --out {out} --latest [read-only]")]
        }
        Err(_) => Vec::new(),
    }
}

fn progress_receipt(
    plan: &GithubActivityPlanReceipt,
    state: &str,
    completed_windows: Vec<String>,
    pending_windows: Vec<String>,
    run_ref: Option<&str>,
    stop_reason: Option<String>,
    receipt_refs: Vec<String>,
) -> GithubActivityProgressReceipt {
    GithubActivityProgressReceipt {
        schema_version: GITHUB_ACTIVITY_PROGRESS_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        shiplog_version: env!("CARGO_PKG_VERSION").to_string(),
        activity_id: plan.activity_id.clone(),
        plan_ref: GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
        actor: plan.actor.clone(),
        repo_owners: plan.repo_owners.clone(),
        profile: plan.profile.clone(),
        state: state.to_string(),
        completed_windows,
        pending_windows,
        active_window: None,
        stop_reason,
        budget_checkpoint: None,
        run_ref: run_ref.map(ToOwned::to_owned),
        receipt_refs,
    }
}

fn activity_stop_reason(err: &anyhow::Error) -> String {
    let budget_exhausted = err
        .chain()
        .any(|cause| cause.to_string().contains("budget exhausted"));
    if budget_exhausted {
        "budget_exhausted".to_string()
    } else {
        "provider_error".to_string()
    }
}

fn api_ledger_receipt(
    plan: &GithubActivityPlanReceipt,
    requests: GithubApiRequestCounts,
    cache: GithubApiCacheCounts,
    rate_limit_snapshots: Vec<GithubRateLimitSnapshot>,
    secondary_limit_events: Vec<GithubSecondaryLimitEvent>,
    stop_reason: Option<String>,
    owner_filter: OwnerFilterLedger,
) -> GithubActivityApiLedgerReceipt {
    GithubActivityApiLedgerReceipt {
        schema_version: GITHUB_ACTIVITY_API_LEDGER_SCHEMA_VERSION.to_string(),
        generated_at: Utc::now().to_rfc3339(),
        shiplog_version: env!("CARGO_PKG_VERSION").to_string(),
        activity_id: plan.activity_id.clone(),
        plan_ref: GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
        progress_ref: GITHUB_ACTIVITY_PROGRESS_FILENAME.to_string(),
        actor: plan.actor.clone(),
        repo_owners: plan.repo_owners.clone(),
        profile: plan.profile.clone(),
        stop_reason,
        github_api: GithubActivityApiLedgerGithub {
            requests,
            cache,
            rate_limit_snapshots,
            secondary_limit_events,
        },
        owner_filter,
        receipt_refs: vec![
            GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
            GITHUB_ACTIVITY_PROGRESS_FILENAME.to_string(),
        ],
    }
}

fn owner_filter_from_notes(
    plan: &GithubActivityPlanReceipt,
    notes: &[String],
) -> OwnerFilterLedger {
    let mut owner_filter = OwnerFilterLedger::from_plan(plan);
    for note in notes {
        if let Some(value) = note.strip_prefix("owner_filter:kept=") {
            owner_filter.kept = parse_owner_counts(value);
        } else if let Some(value) = note.strip_prefix("owner_filter:dropped=") {
            owner_filter.dropped = parse_owner_drops(value);
        }
    }
    owner_filter
}

fn parse_owner_counts(value: &str) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    if value == "none" || value.trim().is_empty() {
        return counts;
    }
    for pair in value.split(',') {
        let Some((owner, count)) = pair.rsplit_once('=') else {
            continue;
        };
        if let Ok(count) = count.parse::<u64>() {
            counts.insert(owner.to_string(), count);
        }
    }
    counts
}

fn parse_owner_drops(value: &str) -> Vec<OwnerFilterDrop> {
    parse_owner_counts(value)
        .into_iter()
        .map(|(owner, count)| OwnerFilterDrop {
            owner,
            count,
            reason: "owner_not_requested".to_string(),
        })
        .collect()
}

fn write_json_receipt<T: Serialize>(path: &Path, label: &str, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value).with_context(|| format!("serialize {label}"))?;
    ensure_no_secret_sentinels(label, &json)?;
    std::fs::write(path, format!("{json}\n")).with_context(|| format!("write {}", path.display()))
}

fn load_progress_if_completed(
    path: &Path,
    plan: &GithubActivityPlanReceipt,
) -> Result<Option<GithubActivityProgressReceipt>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    ensure_no_secret_sentinels(GITHUB_ACTIVITY_PROGRESS_FILENAME, &text)?;
    let progress: GithubActivityProgressReceipt =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    if progress.schema_version == GITHUB_ACTIVITY_PROGRESS_SCHEMA_VERSION
        && progress.activity_id == plan.activity_id
        && progress.profile == plan.profile
        && progress.state == "completed"
    {
        Ok(Some(progress))
    } else {
        Ok(None)
    }
}

fn write_posture(writes: bool) -> &'static str {
    if writes { "writes" } else { "read-only" }
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

fn github_activity_repo_owners(config: &ShiplogConfig) -> Vec<String> {
    if !config.github_activity.repo_owners.is_empty() {
        return normalized_owner_list(&config.github_activity.repo_owners);
    }
    config
        .sources
        .github
        .as_ref()
        .map(|source| normalized_owner_list(&source.repo_owners))
        .unwrap_or_default()
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

struct GithubActivityExecution {
    since: NaiveDate,
    until: NaiveDate,
    mode: String,
    include_reviews: bool,
    no_details: bool,
    throttle_ms: u64,
    api_base: String,
    cache_dir: Option<PathBuf>,
}

#[derive(Deserialize, Serialize)]
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

impl GithubActivityPlanReceipt {
    fn profile_enum(&self) -> Result<GithubActivityProfile> {
        parse_activity_profile(Some(&self.profile))?.ok_or_else(|| {
            anyhow::anyhow!(
                "GitHub activity plan profile is empty; expected scout, authored, or full"
            )
        })
    }

    fn window_ids(&self) -> Vec<String> {
        self.windows
            .iter()
            .map(|window| window.window_id.clone())
            .collect()
    }
}

#[derive(Deserialize, Serialize)]
struct GithubActivityPlanWindow {
    window_id: String,
    since: String,
    until: String,
    granularity: String,
    query_kinds: Vec<String>,
    queries: Vec<GithubActivityPlanQuery>,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityPlanQuery {
    query_kind: String,
    search_query: String,
    estimated_search_requests: u64,
    estimated_core_requests: u64,
    estimated_review_requests: u64,
    dense_window_risk: String,
    cache_reuse: String,
}

#[derive(Default, Deserialize, Serialize)]
struct GithubActivityEstimatedTotals {
    search_requests: u64,
    core_requests: u64,
    review_requests: u64,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityBudgetPolicy {
    max_search_requests: u64,
    max_core_requests: u64,
    max_search_per_minute: u64,
    on_exhausted: String,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityNextAction {
    command: String,
    writes: bool,
    reason: String,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityApiLedgerReceipt {
    schema_version: String,
    generated_at: String,
    shiplog_version: String,
    activity_id: String,
    plan_ref: String,
    progress_ref: String,
    actor: String,
    repo_owners: Vec<String>,
    profile: String,
    stop_reason: Option<String>,
    github_api: GithubActivityApiLedgerGithub,
    owner_filter: OwnerFilterLedger,
    receipt_refs: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityApiLedgerGithub {
    requests: GithubApiRequestCounts,
    cache: GithubApiCacheCounts,
    rate_limit_snapshots: Vec<GithubRateLimitSnapshot>,
    secondary_limit_events: Vec<GithubSecondaryLimitEvent>,
}

#[derive(Deserialize, Serialize)]
struct OwnerFilterLedger {
    requested_owners: Vec<String>,
    query_strategy: String,
    kept: BTreeMap<String, u64>,
    dropped: Vec<OwnerFilterDrop>,
}

impl OwnerFilterLedger {
    fn from_plan(plan: &GithubActivityPlanReceipt) -> Self {
        Self {
            requested_owners: plan.repo_owners.clone(),
            query_strategy: plan.query_strategy.clone(),
            kept: BTreeMap::new(),
            dropped: Vec::new(),
        }
    }

    fn from_coverage(plan: &GithubActivityPlanReceipt, coverage: &CoverageManifest) -> Self {
        let notes = coverage
            .slices
            .iter()
            .flat_map(|slice| slice.notes.iter().cloned())
            .collect::<Vec<_>>();
        owner_filter_from_notes(plan, &notes)
    }
}

#[derive(Deserialize, Serialize)]
struct OwnerFilterDrop {
    owner: String,
    count: u64,
    reason: String,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityProgressReceipt {
    schema_version: String,
    generated_at: String,
    shiplog_version: String,
    activity_id: String,
    plan_ref: String,
    actor: String,
    repo_owners: Vec<String>,
    profile: String,
    state: String,
    completed_windows: Vec<String>,
    pending_windows: Vec<String>,
    active_window: Option<GithubActivityProgressWindow>,
    stop_reason: Option<String>,
    budget_checkpoint: Option<GithubActivityProgressBudget>,
    run_ref: Option<String>,
    receipt_refs: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityProgressWindow {
    window_id: String,
    query_kind: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct GithubActivityProgressBudget {
    search_requests: u64,
    core_requests: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activity_config() -> Result<ShiplogConfig> {
        toml::from_str(
            r#"
[shiplog]
config_version = 1

[defaults]
out = "./out/activity"

[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2026-01-01"
until = "2026-03-01"
include_reviews = true
cache_dir = "./out/activity/.cache"

[github_activity.budget]
max_search_per_minute = 24
max_search_requests = 12
max_core_requests = 34

[sources.github]
enabled = true
mode = "created"
throttle_ms = 100
api_base = "https://api.github.test"
"#,
        )
        .context("parse test config")
    }

    #[test]
    fn activity_execution_applies_profiles() -> Result<()> {
        let config = activity_config()?;
        let base_dir = Path::new(".");
        let out_dir = Path::new("./out/activity");

        let scout = activity_execution(&config, base_dir, out_dir, GithubActivityProfile::Scout)?;
        assert!(!scout.include_reviews);
        assert!(scout.no_details);
        assert_eq!(scout.throttle_ms, 2500);
        assert_eq!(scout.api_base, "https://api.github.test");
        assert_eq!(scout.mode, "created");

        let authored =
            activity_execution(&config, base_dir, out_dir, GithubActivityProfile::Authored)?;
        assert!(!authored.include_reviews);
        assert!(!authored.no_details);

        let full = activity_execution(&config, base_dir, out_dir, GithubActivityProfile::Full)?;
        assert!(full.include_reviews);
        assert!(!full.no_details);

        Ok(())
    }

    #[test]
    fn activity_next_actions_route_profile_progression() -> Result<()> {
        let config = activity_config()?;
        let out_dir = Path::new("./out/activity");

        let scout = build_plan(&config, Some(GithubActivityProfile::Scout))?;
        let scout_actions = activity_next_actions(&scout, out_dir);
        assert_eq!(scout_actions.len(), 1);
        assert!(scout_actions[0].contains("--profile authored"));
        assert!(scout_actions[0].contains("[writes]"));

        let authored = build_plan(&config, Some(GithubActivityProfile::Authored))?;
        let authored_actions = activity_next_actions(&authored, out_dir);
        assert_eq!(authored_actions.len(), 1);
        assert!(authored_actions[0].contains("--profile full"));
        assert!(authored_actions[0].contains("[writes]"));

        let full = build_plan(&config, Some(GithubActivityProfile::Full))?;
        let full_actions = activity_next_actions(&full, out_dir);
        assert_eq!(full_actions.len(), 1);
        assert!(full_actions[0].contains("shiplog status"));
        assert!(full_actions[0].contains("[read-only]"));

        Ok(())
    }

    #[test]
    fn completed_progress_resume_matches_same_activity_only() -> Result<()> {
        let config = activity_config()?;
        let scout = build_plan(&config, Some(GithubActivityProfile::Scout))?;
        let temp = tempfile::tempdir().context("create temp dir")?;
        let progress_path = temp.path().join(GITHUB_ACTIVITY_PROGRESS_FILENAME);
        let progress = progress_receipt(
            &scout,
            "completed",
            scout.window_ids(),
            Vec::new(),
            Some("run_123"),
            None,
            vec![GITHUB_ACTIVITY_PLAN_FILENAME.to_string()],
        );
        write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;

        let loaded = load_progress_if_completed(&progress_path, &scout)?
            .ok_or_else(|| anyhow::anyhow!("expected completed progress to match"))?;
        assert_eq!(loaded.profile, "scout");
        assert_eq!(loaded.state, "completed");
        assert_eq!(loaded.run_ref.as_deref(), Some("run_123"));

        let authored = build_plan(&config, Some(GithubActivityProfile::Authored))?;
        assert!(load_progress_if_completed(&progress_path, &authored)?.is_none());

        Ok(())
    }

    #[test]
    fn api_ledger_records_requests_cache_and_owner_filter() -> Result<()> {
        let config = activity_config()?;
        let full = build_plan(&config, Some(GithubActivityProfile::Full))?;
        let ledger = api_ledger_receipt(
            &full,
            GithubApiRequestCounts { search: 4, core: 7 },
            GithubApiCacheCounts {
                search_probe: shiplog::ingest::github::GithubApiCachePhaseCounts {
                    fresh_hits: 1,
                    stale_hits: 0,
                    misses: 2,
                },
                search_page: shiplog::ingest::github::GithubApiCachePhaseCounts {
                    fresh_hits: 3,
                    stale_hits: 0,
                    misses: 4,
                },
                pull_detail: shiplog::ingest::github::GithubApiCachePhaseCounts {
                    fresh_hits: 5,
                    stale_hits: 1,
                    misses: 6,
                },
                review_page: shiplog::ingest::github::GithubApiCachePhaseCounts {
                    fresh_hits: 7,
                    stale_hits: 0,
                    misses: 8,
                },
            },
            vec![GithubRateLimitSnapshot {
                resource: "search".to_string(),
                limit: 30,
                remaining: 24,
                used: Some(6),
                reset_at: Some("2026-05-19T00:01:00+00:00".to_string()),
                observed_at: "2026-05-19T00:00:00+00:00".to_string(),
            }],
            vec![GithubSecondaryLimitEvent {
                resource: "search".to_string(),
                status: 429,
                category: "secondary_rate_limit".to_string(),
                retry_after_seconds: Some(30),
                observed_at: "2026-05-19T00:00:00+00:00".to_string(),
            }],
            None,
            owner_filter_from_notes(
                &full,
                &[
                    "owner_filter:kept=EffortlessMetrics=2,EffortlessSteven=1".to_string(),
                    "owner_filter:dropped=OtherOrg=3".to_string(),
                ],
            ),
        );

        assert_eq!(ledger.schema_version, "github.activity.api-ledger.v1");
        assert_eq!(ledger.profile, "full");
        assert_eq!(ledger.github_api.requests.search, 4);
        assert_eq!(ledger.github_api.requests.core, 7);
        assert_eq!(ledger.github_api.cache.pull_detail.stale_hits, 1);
        assert_eq!(ledger.github_api.rate_limit_snapshots[0].remaining, 24);
        assert_eq!(
            ledger.github_api.secondary_limit_events[0].category,
            "secondary_rate_limit"
        );
        assert_eq!(ledger.owner_filter.kept.get("EffortlessMetrics"), Some(&2));
        assert_eq!(ledger.owner_filter.dropped.len(), 1);
        assert_eq!(ledger.owner_filter.dropped[0].owner, "OtherOrg");
        assert_eq!(ledger.owner_filter.dropped[0].reason, "owner_not_requested");
        assert_eq!(
            ledger.receipt_refs,
            vec![
                "github.activity.plan.json".to_string(),
                "github.activity.progress.json".to_string()
            ]
        );

        Ok(())
    }
}
