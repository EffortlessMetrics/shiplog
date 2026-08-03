use anyhow::{Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use shiplog::coverage::month_windows;
use shiplog::ingest::github::{
    GithubApiBudget, GithubApiCacheCounts, GithubApiCachePhaseCounts, GithubApiRequestCounts,
    GithubRateLimitSnapshot, GithubSecondaryLimitEvent,
};
use shiplog::merge::{ConflictResolution, merge_ingest_outputs};
use shiplog::ports::IngestOutput;
use shiplog::schema::coverage::{CoverageManifest, TimeWindow};
use shiplog::schema::event::EventEnvelope;
use shiplog::schema::freshness::SourceFreshness;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};

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

pub(super) fn run_status(args: GithubActivityStatusArgs) -> Result<()> {
    let receipts = load_activity_receipts(&args.config, args.out.as_deref())?;

    println!(
        "GitHub activity status: {}",
        github_activity_status_label(
            receipts.plan.as_ref(),
            receipts.progress.as_ref(),
            receipts.api_ledger.as_ref()
        )
    );
    println!("Out: {}", display_path_for_cli(&receipts.out_dir));
    println!("Provider calls: none (receipt status)");
    println!("Writes: none");

    if let Some(plan) = receipts.plan.as_ref() {
        println!("Plan:");
        println!("- {}", display_path_for_cli(&receipts.plan_path));
        println!("- actor: {}", plan.actor);
        if plan.repo_owners.is_empty() {
            println!("- repository owners: actor-wide (no owner filter requested)");
        } else {
            println!("- repository owners: {}", plan.repo_owners.join(", "));
        }
        println!("- profile: {}", plan.profile);
        println!("- windows: {}", plan.windows.len());
        println!(
            "- estimated requests: search {}, core {}, review {}",
            plan.estimated_totals.search_requests,
            plan.estimated_totals.core_requests,
            plan.estimated_totals.review_requests
        );
    } else {
        println!("Plan:");
        println!("- missing {}", display_path_for_cli(&receipts.plan_path));
    }

    if let Some(progress) = receipts.progress.as_ref() {
        println!("Progress:");
        println!("- {}", display_path_for_cli(&receipts.progress_path));
        println!("- state: {}", progress.state);
        println!("- completed windows: {}", progress.completed_windows.len());
        println!("- pending windows: {}", progress.pending_windows.len());
        if let Some(stop_reason) = progress.stop_reason.as_deref() {
            println!("- stop reason: {stop_reason}");
        }
        if let Some(run_ref) = progress.run_ref.as_deref() {
            println!("- run: {run_ref}");
        }
        if let Some(checkpoint) = progress.budget_checkpoint.as_ref() {
            println!(
                "- budget checkpoint: search {}, core {}",
                checkpoint.search_requests, checkpoint.core_requests
            );
        }
    } else {
        println!("Progress:");
        println!(
            "- missing {}",
            display_path_for_cli(&receipts.progress_path)
        );
    }

    if let Some(api_ledger) = receipts.api_ledger.as_ref() {
        println!("API:");
        println!("- {}", display_path_for_cli(&receipts.api_ledger_path));
        print_api_request_summary(receipts.plan.as_ref(), api_ledger);
        print_cache_summary(api_ledger);
        print_owner_filter_summary(api_ledger);
        if let Some(stop_reason) = api_ledger.stop_reason.as_deref() {
            println!("- stop reason: {stop_reason}");
        }
    } else {
        println!("API:");
        println!(
            "- missing {}",
            display_path_for_cli(&receipts.api_ledger_path)
        );
    }

    println!("Next:");
    for action in activity_status_next_actions(
        &args.config,
        &receipts.out_dir,
        receipts.plan.as_ref(),
        receipts.progress.as_ref(),
        receipts.api_ledger.as_ref(),
    ) {
        println!("- {action}");
    }

    println!("Receipts:");
    for receipt in activity_status_receipts(
        &receipts.plan_path,
        &receipts.progress_path,
        &receipts.api_ledger_path,
    ) {
        println!("- {}", display_path_for_cli(&receipt));
    }

    Ok(())
}

pub(super) fn run_report(args: GithubActivityStatusArgs) -> Result<()> {
    let receipts = load_activity_receipts(&args.config, args.out.as_deref())?;

    let report_label = github_activity_report_label(
        receipts.plan.as_ref(),
        receipts.progress.as_ref(),
        receipts.api_ledger.as_ref(),
    );
    println!("GitHub activity report: {report_label}");
    println!("Out: {}", display_path_for_cli(&receipts.out_dir));
    println!("Provider calls: none (receipt report)");

    match receipts.plan.as_ref() {
        Some(plan) => {
            println!("Scope:");
            println!("- actor: {}", plan.actor);
            if plan.repo_owners.is_empty() {
                println!("- repository owners: actor-wide (no owner filter requested)");
            } else {
                println!("- repository owners: {}", plan.repo_owners.join(", "));
            }
            println!("- query strategy: {}", plan.query_strategy);
            println!("- profile: {}", plan.profile);
            println!("- period: {} to {}", plan.since, plan.until);
            println!("- windows: {}", plan.windows.len());
        }
        None => {
            println!("Scope:");
            println!("- missing {}", display_path_for_cli(&receipts.plan_path));
        }
    }

    println!("Execution:");
    if let Some(progress) = receipts.progress.as_ref() {
        println!("- state: {}", progress.state);
        println!("- completed windows: {}", progress.completed_windows.len());
        println!("- pending windows: {}", progress.pending_windows.len());
        if let Some(run_ref) = progress.run_ref.as_deref() {
            println!("- run: {run_ref}");
        }
        if let Some(stop_reason) = progress.stop_reason.as_deref() {
            println!("- stop reason: {stop_reason}");
        }
    } else {
        println!(
            "- missing {}",
            display_path_for_cli(&receipts.progress_path)
        );
    }

    println!("API Budget:");
    if let Some(api_ledger) = receipts.api_ledger.as_ref() {
        print_api_request_summary(receipts.plan.as_ref(), api_ledger);
        print_cache_summary(api_ledger);
        println!(
            "- rate-limit snapshots: {}",
            api_ledger.github_api.rate_limit_snapshots.len()
        );
        println!(
            "- secondary-limit events: {}",
            api_ledger.github_api.secondary_limit_events.len()
        );
        if let Some(stop_reason) = api_ledger.stop_reason.as_deref() {
            println!("- stop reason: {stop_reason}");
        }
    } else {
        println!(
            "- missing {}",
            display_path_for_cli(&receipts.api_ledger_path)
        );
    }

    println!("Owner Filter:");
    if let Some(api_ledger) = receipts.api_ledger.as_ref() {
        print_owner_filter_summary(api_ledger);
    } else if let Some(plan) = receipts.plan.as_ref() {
        if plan.repo_owners.is_empty() {
            println!("- owner filter: actor-wide");
        } else {
            println!("- requested owners: {}", plan.repo_owners.join(", "));
            println!("- kept owners: unknown (API ledger missing)");
            println!("- dropped owners: unknown (API ledger missing)");
        }
    } else {
        println!("- unknown (plan missing)");
    }

    println!("Receipts:");
    for receipt in activity_status_receipts(
        &receipts.plan_path,
        &receipts.progress_path,
        &receipts.api_ledger_path,
    ) {
        println!("- {}", display_path_for_cli(&receipt));
    }

    println!("Next:");
    for action in activity_status_next_actions(
        &args.config,
        &receipts.out_dir,
        receipts.plan.as_ref(),
        receipts.progress.as_ref(),
        receipts.api_ledger.as_ref(),
    ) {
        println!("- {action}");
    }

    if let (Some(plan), Some(progress), Some(api_ledger)) = (
        receipts.plan.as_ref(),
        receipts.progress.as_ref(),
        receipts.api_ledger.as_ref(),
    ) {
        let report = build_activity_report_receipt(
            plan,
            progress,
            api_ledger,
            &receipts.out_dir,
            None,
            Vec::new(),
        );
        let report_json_path = receipts.out_dir.join(GITHUB_ACTIVITY_REPORT_FILENAME);
        let report_md_path = receipts
            .out_dir
            .join(GITHUB_ACTIVITY_REPORT_MARKDOWN_FILENAME);
        write_json_receipt(&report_json_path, GITHUB_ACTIVITY_REPORT_FILENAME, &report)?;
        write_activity_report_markdown(&report_md_path, &report)?;
        println!("Writes:");
        println!("- {}", display_path_for_cli(&report_json_path));
        println!("- {}", display_path_for_cli(&report_md_path));
    } else {
        println!("Writes: none (missing plan, progress, or API ledger)");
    }

    Ok(())
}

pub(super) fn run_merge(args: GithubActivityStatusArgs) -> Result<()> {
    let receipts = load_activity_receipts(&args.config, args.out.as_deref())?;
    let plan = receipts.plan.as_ref().ok_or_else(|| {
        anyhow::anyhow!("cannot merge GitHub activity without github.activity.plan.json")
    })?;
    let progress = receipts.progress.as_ref().ok_or_else(|| {
        anyhow::anyhow!("cannot merge GitHub activity without github.activity.progress.json")
    })?;
    if progress.state != "completed" {
        anyhow::bail!(
            "cannot merge GitHub activity while progress state is {}; rerun with --resume until it completes",
            progress.state
        );
    }
    let run_ref = progress
        .run_ref
        .as_deref()
        .ok_or_else(|| anyhow::anyhow!("cannot merge GitHub activity without progress.run_ref"))?;
    let run_ref = validate_github_activity_run_ref(run_ref)?;
    let api_ledger = receipts.api_ledger.as_ref().ok_or_else(|| {
        anyhow::anyhow!("cannot merge GitHub activity without github.activity.api-ledger.json")
    })?;

    let source_run_dir = receipts.out_dir.join(run_ref);
    let final_dir = receipts.out_dir.join("final");
    std::fs::create_dir_all(&final_dir)
        .with_context(|| format!("create {}", final_dir.display()))?;

    let mut outputs = Vec::new();
    copy_activity_final_output(&source_run_dir, &final_dir, "packet.md", true, &mut outputs)?;
    copy_activity_final_output(
        &source_run_dir,
        &final_dir,
        "intake.report.json",
        false,
        &mut outputs,
    )?;
    copy_activity_final_output(
        &source_run_dir,
        &final_dir,
        "coverage.manifest.json",
        false,
        &mut outputs,
    )?;
    copy_activity_final_output(
        &source_run_dir,
        &final_dir,
        "ledger.events.jsonl",
        false,
        &mut outputs,
    )?;
    copy_activity_final_output(
        &receipts.out_dir,
        &final_dir,
        GITHUB_ACTIVITY_API_LEDGER_FILENAME,
        true,
        &mut outputs,
    )?;

    let report_path = final_dir.join(GITHUB_ACTIVITY_REPORT_FILENAME);
    let report = build_activity_report_receipt(
        plan,
        progress,
        api_ledger,
        &receipts.out_dir,
        Some(&final_dir),
        outputs.clone(),
    );
    write_json_receipt(&report_path, GITHUB_ACTIVITY_REPORT_FILENAME, &report)?;
    outputs.push(GithubActivityFinalOutput {
        label: "activity_report".to_string(),
        path: display_path_for_cli(&report_path),
    });

    println!("GitHub activity merge written:");
    println!("- source run: {}", display_path_for_cli(&source_run_dir));
    println!("- final: {}", display_path_for_cli(&final_dir));
    println!("Provider calls: none (receipt merge)");
    println!("Share rendering: none");
    println!("Writes:");
    for output in &outputs {
        println!("- {}: {}", output.label, output.path);
    }
    println!("Next:");
    println!(
        "- shiplog status --out {} --latest [read-only]",
        quote_cli_value(&display_path_for_cli(&receipts.out_dir))
    );
    println!(
        "- shiplog open packet --out {} --run final [read-only]",
        quote_cli_value(&display_path_for_cli(&receipts.out_dir))
    );

    Ok(())
}

fn validate_github_activity_run_ref(run_ref: &str) -> Result<&str> {
    if run_ref.is_empty() {
        anyhow::bail!("cannot merge GitHub activity without progress.run_ref");
    }

    let mut normal_component_count = 0usize;
    for component in Path::new(run_ref).components() {
        match component {
            Component::Normal(_) => normal_component_count += 1,
            _ => {
                anyhow::bail!(
                    "cannot merge GitHub activity with unsafe progress.run_ref; expected exactly one normal segment"
                );
            }
        }
    }

    if normal_component_count != 1 {
        anyhow::bail!(
            "cannot merge GitHub activity with unsafe progress.run_ref; expected exactly one normal segment"
        );
    }

    Ok(run_ref)
}

fn load_activity_receipts(
    config_path: &Path,
    out: Option<&Path>,
) -> Result<GithubActivityReceipts> {
    let config = load_config_for_command(config_path)?;
    ensure_supported_config_version(&config)?;

    let base_dir = config_base_dir(config_path);
    let out_dir = out
        .map(|path| resolve_config_path(Path::new("."), path))
        .unwrap_or_else(|| github_activity_default_out(&config, &base_dir));

    let plan_path = out_dir.join(GITHUB_ACTIVITY_PLAN_FILENAME);
    let progress_path = out_dir.join(GITHUB_ACTIVITY_PROGRESS_FILENAME);
    let api_ledger_path = out_dir.join(GITHUB_ACTIVITY_API_LEDGER_FILENAME);

    let plan: Option<GithubActivityPlanReceipt> =
        read_json_receipt_if_present(&plan_path, GITHUB_ACTIVITY_PLAN_FILENAME)?;
    let progress: Option<GithubActivityProgressReceipt> =
        read_json_receipt_if_present(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME)?;
    let api_ledger: Option<GithubActivityApiLedgerReceipt> =
        read_json_receipt_if_present(&api_ledger_path, GITHUB_ACTIVITY_API_LEDGER_FILENAME)?;

    Ok(GithubActivityReceipts {
        out_dir,
        plan_path,
        progress_path,
        api_ledger_path,
        plan,
        progress,
        api_ledger,
    })
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
    let credential = resolve_github_credential(&execution.api_base)?;
    let auth = GithubActivityAuthMetadata {
        source: credential.metadata().source.label().to_string(),
        host: credential.metadata().host.clone(),
        account: credential.metadata().account.clone(),
    };
    let start_state = match plan.profile_enum()? {
        GithubActivityProfile::Scout => "scouting",
        GithubActivityProfile::Authored | GithubActivityProfile::Full => "running",
    };
    let existing_progress = if args.resume {
        load_progress_if_compatible(&progress_path, &plan)?
    } else {
        None
    };
    let mut api_accumulator = if args.resume {
        read_json_receipt_if_present::<GithubActivityApiLedgerReceipt>(
            &api_ledger_path,
            GITHUB_ACTIVITY_API_LEDGER_FILENAME,
        )?
        .filter(|ledger| ledger.activity_id == plan.activity_id && ledger.profile == plan.profile)
        .map(GithubActivityApiAccumulator::from_ledger)
        .unwrap_or_default()
    } else {
        GithubActivityApiAccumulator::default()
    };

    let (mut completed_windows, mut window_outputs) =
        load_completed_window_outputs(&out_dir, &plan, existing_progress.as_ref())?;
    write_json_receipt(
        &progress_path,
        GITHUB_ACTIVITY_PROGRESS_FILENAME,
        &progress_receipt(
            &plan,
            start_state,
            completed_windows.clone(),
            pending_window_ids(&plan, &completed_windows),
            None,
            None,
            vec![GITHUB_ACTIVITY_PLAN_FILENAME.to_string()],
        ),
    )?;

    for window in pending_plan_windows(&plan, &completed_windows) {
        let pending_windows = pending_window_ids(&plan, &completed_windows);
        let mut progress = progress_receipt(
            &plan,
            start_state,
            completed_windows.clone(),
            pending_windows.clone(),
            None,
            None,
            vec![GITHUB_ACTIVITY_PLAN_FILENAME.to_string()],
        );
        progress.active_window = Some(GithubActivityProgressWindow {
            window_id: window.window_id.clone(),
            query_kind: None,
        });
        write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;

        let window_range = plan_window_time_window(window)?;
        let mut ing = make_github_ingestor(
            &plan.actor,
            window_range.since,
            window_range.until,
            &execution.mode,
            plan.repo_owners.clone(),
            execution.include_reviews,
            execution.no_details,
            execution.throttle_ms,
            Some(credential.secret().to_owned()),
            &execution.api_base,
            execution.cache_dir.clone(),
        )
        .context("create GitHub activity ingestor")?;
        if let Some(cache_ttl_days) = execution.cache_ttl_days {
            ing = ing.with_cache_ttl(Duration::days(cache_ttl_days));
        }
        let ing = ing.with_api_budget(api_accumulator.remaining_budget(&plan.budget_policy));

        let ingest = match ing.ingest() {
            Ok(ingest) => ingest,
            Err(err) => {
                api_accumulator.add_run(
                    ing.api_request_counts(),
                    ing.api_cache_counts(),
                    ing.rate_limit_snapshots(),
                    ing.secondary_limit_events(),
                );
                let stop_reason = activity_stop_reason(&err);
                let mut progress = progress_receipt(
                    &plan,
                    "checkpointed",
                    completed_windows.clone(),
                    pending_windows,
                    None,
                    Some(stop_reason.clone()),
                    vec![
                        GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
                        GITHUB_ACTIVITY_API_LEDGER_FILENAME.to_string(),
                    ],
                );
                progress.active_window = Some(GithubActivityProgressWindow {
                    window_id: window.window_id.clone(),
                    query_kind: None,
                });
                progress.budget_checkpoint = Some(GithubActivityProgressBudget {
                    search_requests: api_accumulator.requests.search,
                    core_requests: api_accumulator.requests.core,
                });
                write_json_receipt(&progress_path, GITHUB_ACTIVITY_PROGRESS_FILENAME, &progress)?;
                let owner_filter = owner_filter_from_outputs(&plan, &window_outputs);
                let api_ledger = api_accumulator.to_ledger(
                    &plan,
                    Some(stop_reason),
                    owner_filter,
                    Some(auth.clone()),
                );
                write_json_receipt(
                    &api_ledger_path,
                    GITHUB_ACTIVITY_API_LEDGER_FILENAME,
                    &api_ledger,
                )?;
                println!(
                    "GitHub activity {} checkpointed after search {}, core {} request(s).",
                    plan.profile, api_accumulator.requests.search, api_accumulator.requests.core
                );
                println!("- {}", display_path_for_cli(&progress_path));
                println!("- {}", display_path_for_cli(&api_ledger_path));
                return Err(err).context("GitHub activity ingest stopped before completion");
            }
        };

        api_accumulator.add_run(
            ing.api_request_counts(),
            ing.api_cache_counts(),
            ing.rate_limit_snapshots(),
            ing.secondary_limit_events(),
        );
        write_window_ingest_output(&out_dir, &plan.profile, &window.window_id, &ingest)
            .with_context(|| format!("write GitHub activity window {}", window.window_id))?;
        window_outputs.push(ingest);
        completed_windows.push(window.window_id.clone());
        write_json_receipt(
            &progress_path,
            GITHUB_ACTIVITY_PROGRESS_FILENAME,
            &progress_receipt(
                &plan,
                start_state,
                completed_windows.clone(),
                pending_window_ids(&plan, &completed_windows),
                None,
                None,
                vec![GITHUB_ACTIVITY_PLAN_FILENAME.to_string()],
            ),
        )?;
    }

    let mut ingest = merge_window_outputs(&window_outputs)?;
    ingest.coverage.window = TimeWindow {
        since: execution.since,
        until: execution.until,
    };
    ingest.coverage.mode = execution.mode.clone();

    let owner_filter = OwnerFilterLedger::from_coverage(&plan, &ingest.coverage);
    let api_ledger = api_accumulator.to_ledger(&plan, None, owner_filter, Some(auth));
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
    copy_activity_run_receipt(&plan_path, &run_dir, GITHUB_ACTIVITY_PLAN_FILENAME)?;
    copy_activity_run_receipt(&progress_path, &run_dir, GITHUB_ACTIVITY_PROGRESS_FILENAME)?;
    copy_activity_run_receipt(
        &api_ledger_path,
        &run_dir,
        GITHUB_ACTIVITY_API_LEDGER_FILENAME,
    )?;

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
    let cache_ttl_days = config.github_activity.cache_ttl_days;
    if cache_ttl_days.is_some_and(|days| days <= 0) {
        anyhow::bail!("github_activity.cache_ttl_days must be greater than zero when set");
    }
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
        cache_ttl_days,
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

fn github_activity_status_label(
    plan: Option<&GithubActivityPlanReceipt>,
    progress: Option<&GithubActivityProgressReceipt>,
    api_ledger: Option<&GithubActivityApiLedgerReceipt>,
) -> &'static str {
    match (plan, progress, api_ledger) {
        (None, _, _) => "Not planned",
        (Some(_), None, _) => "Planned",
        (Some(_), Some(progress), None) if progress.state == "completed" => {
            "Completed with missing API ledger"
        }
        (Some(_), Some(progress), _) if progress.state == "completed" => "Completed",
        (Some(_), Some(progress), _) if progress.state == "checkpointed" => "Checkpointed",
        (Some(_), Some(progress), _)
            if progress.state == "running" || progress.state == "scouting" =>
        {
            "In progress"
        }
        (Some(_), Some(_), _) => "Needs inspection",
    }
}

fn github_activity_report_label(
    plan: Option<&GithubActivityPlanReceipt>,
    progress: Option<&GithubActivityProgressReceipt>,
    api_ledger: Option<&GithubActivityApiLedgerReceipt>,
) -> &'static str {
    match (plan, progress, api_ledger) {
        (None, _, _) => "Missing plan",
        (Some(_), None, _) => "Missing progress",
        (Some(_), Some(_), None) => "Missing API ledger",
        (Some(_), Some(progress), Some(_)) if progress.state == "completed" => "Available",
        (Some(_), Some(progress), Some(_)) if progress.state == "checkpointed" => "Checkpointed",
        (Some(_), Some(_), Some(_)) => "Partial",
    }
}

fn print_api_request_summary(
    plan: Option<&GithubActivityPlanReceipt>,
    api_ledger: &GithubActivityApiLedgerReceipt,
) {
    if let Some(plan) = plan {
        println!(
            "- requests: search {}/{}, core {}/{}",
            api_ledger.github_api.requests.search,
            plan.budget_policy.max_search_requests,
            api_ledger.github_api.requests.core,
            plan.budget_policy.max_core_requests
        );
    } else {
        println!(
            "- requests: search {}, core {}",
            api_ledger.github_api.requests.search, api_ledger.github_api.requests.core
        );
    }
}

fn print_cache_summary(api_ledger: &GithubActivityApiLedgerReceipt) {
    println!(
        "- cache: search_probe {}, search_page {}, pull_detail {}, review_page {}",
        cache_phase_summary(api_ledger.github_api.cache.search_probe),
        cache_phase_summary(api_ledger.github_api.cache.search_page),
        cache_phase_summary(api_ledger.github_api.cache.pull_detail),
        cache_phase_summary(api_ledger.github_api.cache.review_page)
    );
}

fn print_owner_filter_summary(api_ledger: &GithubActivityApiLedgerReceipt) {
    if api_ledger.owner_filter.requested_owners.is_empty() {
        println!("- owner filter: actor-wide");
    } else {
        println!(
            "- owner filter: requested {}",
            api_ledger.owner_filter.requested_owners.join(", ")
        );
    }
    println!(
        "- kept owners: {}",
        owner_counts_summary(&api_ledger.owner_filter.kept)
    );
    println!(
        "- dropped owners: {}",
        owner_drops_summary(&api_ledger.owner_filter.dropped)
    );
}

fn activity_status_next_actions(
    config_path: &Path,
    out_dir: &Path,
    plan: Option<&GithubActivityPlanReceipt>,
    progress: Option<&GithubActivityProgressReceipt>,
    api_ledger: Option<&GithubActivityApiLedgerReceipt>,
) -> Vec<String> {
    let config = quote_cli_value(&display_path_for_cli(config_path));
    let out = quote_cli_value(&display_path_for_cli(out_dir));

    let Some(plan) = plan else {
        return vec![format!(
            "shiplog github activity plan --config {config} --out {out} [writes]"
        )];
    };

    let Ok(profile) = plan.profile_enum() else {
        return vec![
            "repair github.activity.plan.json profile before running activity [manual]".to_string(),
        ];
    };

    let Some(progress) = progress else {
        return vec![activity_status_run_command(config_path, out_dir, profile)];
    };

    if progress.state == "completed" && api_ledger.is_none() {
        return vec![format!(
            "{} [writes]",
            activity_status_run_base_command(config_path, out_dir, profile)
        )];
    }

    match progress.state.as_str() {
        "completed" => match profile {
            GithubActivityProfile::Scout => vec![format!(
                "{} [writes]",
                activity_status_run_base_command(
                    config_path,
                    out_dir,
                    GithubActivityProfile::Authored
                )
            )],
            GithubActivityProfile::Authored => vec![format!(
                "{} [writes]",
                activity_status_run_base_command(config_path, out_dir, GithubActivityProfile::Full)
            )],
            GithubActivityProfile::Full => vec![
                format!("shiplog github activity status --out {out} [read-only]"),
                format!("shiplog github activity report --out {out} [read-only]"),
                format!("shiplog github activity merge --out {out} [writes]"),
            ],
        },
        "checkpointed" | "running" | "scouting" => {
            vec![activity_status_run_command(config_path, out_dir, profile)]
        }
        _ => vec![format!(
            "inspect {} before continuing [read-only]",
            display_path_for_cli(&out_dir.join(GITHUB_ACTIVITY_PROGRESS_FILENAME))
        )],
    }
}

fn activity_status_run_command(
    config_path: &Path,
    out_dir: &Path,
    profile: GithubActivityProfile,
) -> String {
    format!(
        "{} [writes]",
        activity_status_run_base_command(config_path, out_dir, profile)
    )
}

fn activity_status_run_base_command(
    config_path: &Path,
    out_dir: &Path,
    profile: GithubActivityProfile,
) -> String {
    let config = quote_cli_value(&display_path_for_cli(config_path));
    let out = quote_cli_value(&display_path_for_cli(out_dir));
    match profile {
        GithubActivityProfile::Scout => {
            format!("shiplog github activity scout --config {config} --out {out} --resume")
        }
        GithubActivityProfile::Authored => format!(
            "shiplog github activity run --config {config} --out {out} --profile authored --resume"
        ),
        GithubActivityProfile::Full => format!(
            "shiplog github activity run --config {config} --out {out} --profile full --resume"
        ),
    }
}

fn activity_status_receipts(
    plan_path: &Path,
    progress_path: &Path,
    api_ledger_path: &Path,
) -> Vec<PathBuf> {
    [plan_path, progress_path, api_ledger_path]
        .into_iter()
        .filter(|path| path.exists())
        .map(Path::to_path_buf)
        .collect()
}

fn build_activity_report_receipt<'a>(
    plan: &'a GithubActivityPlanReceipt,
    progress: &'a GithubActivityProgressReceipt,
    api_ledger: &'a GithubActivityApiLedgerReceipt,
    out_dir: &Path,
    final_dir: Option<&Path>,
    final_outputs: Vec<GithubActivityFinalOutput>,
) -> GithubActivityReportReceipt<'a> {
    let run_ref = progress
        .run_ref
        .clone()
        .unwrap_or_else(|| "not_generated".to_string());
    let source_run_dir = progress
        .run_ref
        .as_deref()
        .map(|run_ref| display_path_for_cli(&out_dir.join(run_ref)))
        .unwrap_or_else(|| "not_generated".to_string());
    let final_dir = final_dir
        .map(display_path_for_cli)
        .unwrap_or_else(|| "not_generated".to_string());
    GithubActivityReportReceipt {
        schema_version: GITHUB_ACTIVITY_REPORT_SCHEMA_VERSION,
        generated_at: Utc::now().to_rfc3339(),
        shiplog_version: env!("CARGO_PKG_VERSION"),
        activity_id: &plan.activity_id,
        actor: &plan.actor,
        repo_owners: &plan.repo_owners,
        query_strategy: &plan.query_strategy,
        profile: &plan.profile,
        state: &progress.state,
        run_ref,
        source_run_dir,
        final_dir,
        final_outputs,
        github_api: &api_ledger.github_api,
        owner_filter: &api_ledger.owner_filter,
        receipt_refs: activity_report_receipt_refs(progress),
    }
}

fn activity_report_receipt_refs(progress: &GithubActivityProgressReceipt) -> Vec<String> {
    let mut refs = vec![
        GITHUB_ACTIVITY_PLAN_FILENAME.to_string(),
        GITHUB_ACTIVITY_PROGRESS_FILENAME.to_string(),
        GITHUB_ACTIVITY_API_LEDGER_FILENAME.to_string(),
    ];
    if let Some(run_ref) = progress.run_ref.as_deref() {
        refs.push(format!("{run_ref}/intake.report.json"));
        refs.push(format!("{run_ref}/coverage.manifest.json"));
    }
    refs
}

fn write_activity_report_markdown(
    path: &Path,
    report: &GithubActivityReportReceipt<'_>,
) -> Result<()> {
    let markdown = render_activity_report_markdown(report);
    ensure_no_secret_sentinels(GITHUB_ACTIVITY_REPORT_MARKDOWN_FILENAME, &markdown)?;
    std::fs::write(path, markdown).with_context(|| format!("write {}", path.display()))
}

fn render_activity_report_markdown(report: &GithubActivityReportReceipt<'_>) -> String {
    let mut out = String::new();
    out.push_str("# GitHub Activity Report\n\n");
    out.push_str("## Scope\n\n");
    out.push_str(&format!("- Actor: {}\n", report.actor));
    if report.repo_owners.is_empty() {
        out.push_str("- Repository owners: actor-wide (no owner filter requested)\n");
    } else {
        out.push_str(&format!(
            "- Repository owners: {}\n",
            report.repo_owners.join(", ")
        ));
    }
    out.push_str(&format!("- Query strategy: {}\n", report.query_strategy));
    out.push_str(&format!("- Profile: {}\n", report.profile));
    out.push_str(&format!("- State: {}\n", report.state));
    out.push_str(&format!("- Run: {}\n\n", report.run_ref));

    out.push_str("## API Budget\n\n");
    out.push_str(&format!(
        "- Requests: search {}, core {}\n",
        report.github_api.requests.search, report.github_api.requests.core
    ));
    out.push_str(&format!(
        "- Search probe cache: {}\n",
        cache_phase_summary(report.github_api.cache.search_probe)
    ));
    out.push_str(&format!(
        "- Search page cache: {}\n",
        cache_phase_summary(report.github_api.cache.search_page)
    ));
    out.push_str(&format!(
        "- Pull detail cache: {}\n",
        cache_phase_summary(report.github_api.cache.pull_detail)
    ));
    out.push_str(&format!(
        "- Review page cache: {}\n",
        cache_phase_summary(report.github_api.cache.review_page)
    ));
    out.push_str(&format!(
        "- Rate-limit snapshots: {}\n",
        report.github_api.rate_limit_snapshots.len()
    ));
    out.push_str(&format!(
        "- Secondary-limit events: {}\n\n",
        report.github_api.secondary_limit_events.len()
    ));

    out.push_str("## Owner Filter\n\n");
    if report.owner_filter.requested_owners.is_empty() {
        out.push_str("- Owner filter: actor-wide\n");
    } else {
        out.push_str(&format!(
            "- Requested owners: {}\n",
            report.owner_filter.requested_owners.join(", ")
        ));
    }
    out.push_str(&format!(
        "- Kept owners: {}\n",
        owner_counts_summary(&report.owner_filter.kept)
    ));
    out.push_str(&format!(
        "- Dropped owners: {}\n\n",
        owner_drops_summary(&report.owner_filter.dropped)
    ));

    out.push_str("## Outputs\n\n");
    out.push_str(&format!(
        "- Source run directory: {}\n",
        report.source_run_dir
    ));
    out.push_str(&format!("- Final directory: {}\n", report.final_dir));
    if report.final_outputs.is_empty() {
        out.push_str("- Final outputs: none\n\n");
    } else {
        out.push_str("- Final outputs:\n");
        for output in &report.final_outputs {
            out.push_str(&format!("  - {}: {}\n", output.label, output.path));
        }
        out.push('\n');
    }

    out.push_str("## Receipts\n\n");
    for receipt in &report.receipt_refs {
        out.push_str(&format!("- `{receipt}`\n"));
    }
    out
}

fn load_completed_window_outputs(
    out_dir: &Path,
    plan: &GithubActivityPlanReceipt,
    progress: Option<&GithubActivityProgressReceipt>,
) -> Result<(Vec<String>, Vec<IngestOutput>)> {
    let Some(progress) = progress else {
        return Ok((Vec::new(), Vec::new()));
    };

    let planned_ids = plan.window_ids().into_iter().collect::<BTreeSet<_>>();
    let mut completed_windows = Vec::new();
    let mut outputs = Vec::new();
    for window_id in &progress.completed_windows {
        if !planned_ids.contains(window_id) {
            continue;
        }
        match read_window_ingest_output(out_dir, &plan.profile, window_id) {
            Ok(output) => {
                completed_windows.push(window_id.clone());
                outputs.push(output);
            }
            Err(_) => {
                // Missing or malformed window receipts are not trusted for
                // resume; leave that window pending so it can be fetched again.
            }
        }
    }
    Ok((completed_windows, outputs))
}

fn pending_plan_windows<'a>(
    plan: &'a GithubActivityPlanReceipt,
    completed_windows: &[String],
) -> Vec<&'a GithubActivityPlanWindow> {
    let completed = completed_windows
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    plan.windows
        .iter()
        .filter(|window| !completed.contains(window.window_id.as_str()))
        .collect()
}

fn pending_window_ids(
    plan: &GithubActivityPlanReceipt,
    completed_windows: &[String],
) -> Vec<String> {
    pending_plan_windows(plan, completed_windows)
        .into_iter()
        .map(|window| window.window_id.clone())
        .collect()
}

fn plan_window_time_window(window: &GithubActivityPlanWindow) -> Result<TimeWindow> {
    let since = NaiveDate::parse_from_str(&window.since, "%Y-%m-%d")
        .with_context(|| format!("parse GitHub activity window since {:?}", window.since))?;
    let until = NaiveDate::parse_from_str(&window.until, "%Y-%m-%d")
        .with_context(|| format!("parse GitHub activity window until {:?}", window.until))?;
    Ok(TimeWindow { since, until })
}

fn activity_window_dir(out_dir: &Path, profile: &str, window_id: &str) -> PathBuf {
    out_dir
        .join("github.activity.windows")
        .join(profile)
        .join(window_id)
}

fn write_window_ingest_output(
    out_dir: &Path,
    profile: &str,
    window_id: &str,
    ingest: &IngestOutput,
) -> Result<()> {
    let window_dir = activity_window_dir(out_dir, profile, window_id);
    std::fs::create_dir_all(&window_dir)
        .with_context(|| format!("create {}", window_dir.display()))?;
    write_activity_events_jsonl(&window_dir.join("ledger.events.jsonl"), &ingest.events)?;
    write_activity_coverage_manifest(&window_dir.join("coverage.manifest.json"), &ingest.coverage)?;
    write_json_receipt(
        &window_dir.join("freshness.json"),
        "github activity window freshness",
        &ingest.freshness,
    )
}

fn write_activity_events_jsonl(path: &Path, events: &[EventEnvelope]) -> Result<()> {
    let mut file =
        std::fs::File::create(path).with_context(|| format!("create {}", path.display()))?;
    for event in events {
        let line = serde_json::to_string(event).context("serialize GitHub activity event")?;
        file.write_all(line.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        file.write_all(b"\n")
            .with_context(|| format!("write {}", path.display()))?;
    }
    Ok(())
}

fn write_activity_coverage_manifest(path: &Path, coverage: &CoverageManifest) -> Result<()> {
    let text =
        serde_json::to_string_pretty(coverage).context("serialize GitHub activity coverage")?;
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))
}

fn read_window_ingest_output(
    out_dir: &Path,
    profile: &str,
    window_id: &str,
) -> Result<IngestOutput> {
    let window_dir = activity_window_dir(out_dir, profile, window_id);
    let events_path = window_dir.join("ledger.events.jsonl");
    let coverage_path = window_dir.join("coverage.manifest.json");
    let freshness_path = window_dir.join("freshness.json");

    let file = std::fs::File::open(&events_path)
        .with_context(|| format!("read {}", events_path.display()))?;
    let mut events = Vec::new();
    for (idx, line) in BufReader::new(file).lines().enumerate() {
        let line =
            line.with_context(|| format!("read line {} from {}", idx + 1, events_path.display()))?;
        if line.trim().is_empty() {
            continue;
        }
        events.push(
            serde_json::from_str::<EventEnvelope>(&line).with_context(|| {
                format!("parse line {} from {}", idx + 1, events_path.display())
            })?,
        );
    }

    let coverage_text = std::fs::read_to_string(&coverage_path)
        .with_context(|| format!("read {}", coverage_path.display()))?;
    let coverage = serde_json::from_str::<CoverageManifest>(&coverage_text)
        .with_context(|| format!("parse {}", coverage_path.display()))?;
    let freshness = if freshness_path.exists() {
        let freshness_text = std::fs::read_to_string(&freshness_path)
            .with_context(|| format!("read {}", freshness_path.display()))?;
        serde_json::from_str::<Vec<SourceFreshness>>(&freshness_text)
            .with_context(|| format!("parse {}", freshness_path.display()))?
    } else {
        Vec::new()
    };

    Ok(IngestOutput {
        events,
        coverage,
        freshness,
    })
}

fn merge_window_outputs(outputs: &[IngestOutput]) -> Result<IngestOutput> {
    match outputs {
        [] => anyhow::bail!("GitHub activity produced no completed window receipts"),
        [single] => Ok(single.clone()),
        _ => merge_ingest_outputs(outputs, ConflictResolution::PreferMostRecent)
            .map(|result| result.ingest_output),
    }
}

fn owner_filter_from_outputs(
    plan: &GithubActivityPlanReceipt,
    outputs: &[IngestOutput],
) -> OwnerFilterLedger {
    let notes = outputs
        .iter()
        .flat_map(|output| output.coverage.slices.iter())
        .flat_map(|slice| slice.notes.iter().cloned())
        .collect::<Vec<_>>();
    owner_filter_from_notes(plan, &notes)
}

fn copy_activity_run_receipt(source_path: &Path, run_dir: &Path, filename: &str) -> Result<()> {
    let destination_path = run_dir.join(filename);
    std::fs::copy(source_path, &destination_path).with_context(|| {
        format!(
            "copy {} to {}",
            source_path.display(),
            destination_path.display()
        )
    })?;
    Ok(())
}

fn copy_activity_final_output(
    source_dir: &Path,
    final_dir: &Path,
    filename: &str,
    required: bool,
    outputs: &mut Vec<GithubActivityFinalOutput>,
) -> Result<()> {
    let source_path = source_dir.join(filename);
    if !source_path.exists() {
        if required {
            anyhow::bail!(
                "cannot merge GitHub activity because required output is missing: {}",
                display_path_for_cli(&source_path)
            );
        }
        return Ok(());
    }
    let destination_path = final_dir.join(filename);
    std::fs::copy(&source_path, &destination_path).with_context(|| {
        format!(
            "copy {} to {}",
            source_path.display(),
            destination_path.display()
        )
    })?;
    outputs.push(GithubActivityFinalOutput {
        label: final_output_label(filename).to_string(),
        path: display_path_for_cli(&destination_path),
    });
    Ok(())
}

fn final_output_label(filename: &str) -> &'static str {
    match filename {
        "packet.md" => "packet",
        "intake.report.json" => "intake_report",
        "coverage.manifest.json" => "coverage",
        "ledger.events.jsonl" => "ledger",
        GITHUB_ACTIVITY_API_LEDGER_FILENAME => "api_ledger",
        GITHUB_ACTIVITY_REPORT_FILENAME => "activity_report",
        _ => "artifact",
    }
}

fn cache_phase_summary(counts: GithubApiCachePhaseCounts) -> String {
    format!(
        "fresh {}, stale {}, misses {}",
        counts.fresh_hits, counts.stale_hits, counts.misses
    )
}

fn owner_counts_summary(counts: &BTreeMap<String, u64>) -> String {
    if counts.is_empty() {
        return "none".to_string();
    }
    counts
        .iter()
        .map(|(owner, count)| format!("{owner}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn owner_drops_summary(drops: &[OwnerFilterDrop]) -> String {
    if drops.is_empty() {
        return "none".to_string();
    }
    drops
        .iter()
        .map(|drop| format!("{}={} ({})", drop.owner, drop.count, drop.reason))
        .collect::<Vec<_>>()
        .join(", ")
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
    auth: Option<GithubActivityAuthMetadata>,
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
        auth,
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

fn read_json_receipt_if_present<T: DeserializeOwned>(
    path: &Path,
    label: &str,
) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    ensure_no_secret_sentinels(label, &text)?;
    serde_json::from_str(&text)
        .with_context(|| format!("parse {}", path.display()))
        .map(Some)
}

fn load_progress_if_completed(
    path: &Path,
    plan: &GithubActivityPlanReceipt,
) -> Result<Option<GithubActivityProgressReceipt>> {
    let Some(progress) = load_progress_if_compatible(path, plan)? else {
        return Ok(None);
    };
    if progress.state == "completed" {
        Ok(Some(progress))
    } else {
        Ok(None)
    }
}

fn load_progress_if_compatible(
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

#[derive(Debug)]
struct GithubActivityExecution {
    since: NaiveDate,
    until: NaiveDate,
    mode: String,
    include_reviews: bool,
    no_details: bool,
    throttle_ms: u64,
    api_base: String,
    cache_dir: Option<PathBuf>,
    cache_ttl_days: Option<i64>,
}

struct GithubActivityReceipts {
    out_dir: PathBuf,
    plan_path: PathBuf,
    progress_path: PathBuf,
    api_ledger_path: PathBuf,
    plan: Option<GithubActivityPlanReceipt>,
    progress: Option<GithubActivityProgressReceipt>,
    api_ledger: Option<GithubActivityApiLedgerReceipt>,
}

#[derive(Default)]
struct GithubActivityApiAccumulator {
    requests: GithubApiRequestCounts,
    cache: GithubApiCacheCounts,
    rate_limit_snapshots: Vec<GithubRateLimitSnapshot>,
    secondary_limit_events: Vec<GithubSecondaryLimitEvent>,
}

impl GithubActivityApiAccumulator {
    fn from_ledger(ledger: GithubActivityApiLedgerReceipt) -> Self {
        Self {
            requests: ledger.github_api.requests,
            cache: ledger.github_api.cache,
            rate_limit_snapshots: ledger.github_api.rate_limit_snapshots,
            secondary_limit_events: ledger.github_api.secondary_limit_events,
        }
    }

    fn add_run(
        &mut self,
        requests: GithubApiRequestCounts,
        cache: GithubApiCacheCounts,
        rate_limit_snapshots: Vec<GithubRateLimitSnapshot>,
        secondary_limit_events: Vec<GithubSecondaryLimitEvent>,
    ) {
        self.requests.search = self.requests.search.saturating_add(requests.search);
        self.requests.core = self.requests.core.saturating_add(requests.core);
        add_cache_counts(&mut self.cache, cache);
        self.rate_limit_snapshots.extend(rate_limit_snapshots);
        self.secondary_limit_events.extend(secondary_limit_events);
    }

    fn remaining_budget(&self, policy: &GithubActivityBudgetPolicy) -> GithubApiBudget {
        GithubApiBudget {
            max_search_requests: Some(
                policy
                    .max_search_requests
                    .saturating_sub(self.requests.search),
            ),
            max_core_requests: Some(policy.max_core_requests.saturating_sub(self.requests.core)),
        }
    }

    fn to_ledger(
        &self,
        plan: &GithubActivityPlanReceipt,
        stop_reason: Option<String>,
        owner_filter: OwnerFilterLedger,
        auth: Option<GithubActivityAuthMetadata>,
    ) -> GithubActivityApiLedgerReceipt {
        api_ledger_receipt(
            plan,
            self.requests,
            self.cache,
            self.rate_limit_snapshots.clone(),
            self.secondary_limit_events.clone(),
            stop_reason,
            owner_filter,
            auth,
        )
    }
}

fn add_cache_counts(target: &mut GithubApiCacheCounts, source: GithubApiCacheCounts) {
    add_cache_phase(&mut target.search_probe, source.search_probe);
    add_cache_phase(&mut target.search_page, source.search_page);
    add_cache_phase(&mut target.pull_detail, source.pull_detail);
    add_cache_phase(&mut target.review_page, source.review_page);
}

fn add_cache_phase(target: &mut GithubApiCachePhaseCounts, source: GithubApiCachePhaseCounts) {
    target.fresh_hits = target.fresh_hits.saturating_add(source.fresh_hits);
    target.stale_hits = target.stale_hits.saturating_add(source.stale_hits);
    target.misses = target.misses.saturating_add(source.misses);
}

#[derive(Clone, Serialize)]
struct GithubActivityFinalOutput {
    label: String,
    path: String,
}

#[derive(Serialize)]
struct GithubActivityReportReceipt<'a> {
    schema_version: &'static str,
    generated_at: String,
    shiplog_version: &'static str,
    activity_id: &'a str,
    actor: &'a str,
    repo_owners: &'a [String],
    query_strategy: &'a str,
    profile: &'a str,
    state: &'a str,
    run_ref: String,
    source_run_dir: String,
    final_dir: String,
    final_outputs: Vec<GithubActivityFinalOutput>,
    github_api: &'a GithubActivityApiLedgerGithub,
    owner_filter: &'a OwnerFilterLedger,
    receipt_refs: Vec<String>,
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
    #[serde(default)]
    auth: Option<GithubActivityAuthMetadata>,
    github_api: GithubActivityApiLedgerGithub,
    owner_filter: OwnerFilterLedger,
    receipt_refs: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
struct GithubActivityAuthMetadata {
    source: String,
    host: String,
    account: Option<String>,
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
cache_ttl_days = 3650

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
        assert_eq!(scout.cache_ttl_days, Some(3650));

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
    fn activity_execution_rejects_non_positive_cache_ttl() -> Result<()> {
        let config: ShiplogConfig = toml::from_str(
            r#"
[shiplog]
config_version = 1

[github_activity]
actor = "octocat"
since = "2026-01-01"
until = "2026-02-01"
cache_ttl_days = 0

[sources.github]
enabled = true
user = "octocat"
"#,
        )
        .context("parse test config")?;

        let err = activity_execution(
            &config,
            Path::new("."),
            Path::new("./out"),
            GithubActivityProfile::Scout,
        )
        .expect_err("non-positive GitHub activity cache TTL should fail");
        assert!(
            err.to_string()
                .contains("github_activity.cache_ttl_days must be greater than zero"),
            "unexpected error: {err:?}"
        );

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
            None,
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

    #[test]
    fn github_activity_run_merge_rejects_empty_run_ref() {
        let err = validate_github_activity_run_ref("").expect_err("expected empty run_ref to fail");
        assert!(
            err.to_string()
                .contains("cannot merge GitHub activity without progress.run_ref"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn github_activity_run_merge_rejects_unsafe_run_ref_shape() {
        let invalid = [
            "..",
            "../run_123",
            "/run_123",
            "run_123/..",
            "run_123/../bad",
            "run_123/nested",
        ];
        for run_ref in invalid {
            let err = validate_github_activity_run_ref(run_ref)
                .expect_err("expected unsafe run_ref shape to fail");
            assert!(
                err
                    .to_string()
                    .contains("cannot merge GitHub activity with unsafe progress.run_ref; expected exactly one normal segment"),
                "unexpected error for {run_ref}: {err:?}"
            );
        }
    }

    #[test]
    fn github_activity_run_merge_accepts_safe_run_ref() {
        let valid = ["run_123", "abc-DEF_456", "2026-07-20"];
        for run_ref in valid {
            assert!(validate_github_activity_run_ref(run_ref).is_ok());
        }
    }

    #[cfg(windows)]
    #[test]
    fn github_activity_run_merge_rejects_windows_shaped_run_refs() {
        for run_ref in [r"C:\run_123", r"run_123\..", r"run_123\nested"] {
            let err = validate_github_activity_run_ref(run_ref)
                .expect_err("expected windows run_ref shape to fail");
            assert!(
                err
                    .to_string()
                    .contains("cannot merge GitHub activity with unsafe progress.run_ref; expected exactly one normal segment"),
                "unexpected error for {run_ref}: {err:?}"
            );
        }
    }
}
