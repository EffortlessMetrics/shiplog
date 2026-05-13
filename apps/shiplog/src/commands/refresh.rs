use anyhow::Context;
use std::path::{Path, PathBuf};

use crate::*;

pub(super) fn handle(
    source: Source,
    out: PathBuf,
    explicit_run_dir: Option<PathBuf>,
    zip: bool,
    redact_key: Option<String>,
    bundle_profile: BundleProfile,
) -> Result<()> {
    let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
    let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());

    // Resolve run directory: explicit --run-dir, or find most recent
    let run_dir = if let Some(rd) = explicit_run_dir {
        if rd == Path::new("latest") {
            find_most_recent_run(&out)?
        } else {
            rd
        }
    } else {
        find_most_recent_run(&out)?
    };

    let cache_path = DeterministicRedactor::cache_path(&run_dir);
    let _ = redactor.load_cache(&cache_path);

    match source {
        Source::Git {
            repo,
            window,
            author,
            include_merges,
        } => {
            let window = resolve_date_window(window)?;
            let ing = make_git_ingestor(&repo, window.since, window.until, author, include_merges);
            let ingest = ing.ingest().context("ingest events")?;
            let window_label = window.window_label();

            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let outputs = engine
                .refresh(
                    ingest,
                    "local",
                    &window_label,
                    &run_dir,
                    zip,
                    &bundle_profile,
                )
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }
        Source::Github {
            user,
            me,
            window,
            mode,
            include_reviews,
            no_details,
            throttle_ms,
            token,
            api_base,
            cache_dir,
            no_cache,
        } => {
            let user = resolve_user_or_me("GitHub", user, me, || {
                discover_github_user(&api_base, token.as_deref())
            })?;
            let window = resolve_date_window(window)?;
            let cache_root = run_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| out.clone());
            let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
            let ing = make_github_ingestor(
                &user,
                window.since,
                window.until,
                &mode,
                include_reviews,
                no_details,
                throttle_ms,
                token,
                &api_base,
                cache_dir,
            )
            .context("create GitHub ingestor")?;
            let ingest = ing.ingest().context("ingest events")?;

            let window_label = window.window_label();

            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }

        Source::Gitlab {
            user,
            me,
            window,
            state,
            instance,
            include_reviews,
            throttle_ms,
            token,
            cache_dir,
            no_cache,
        } => {
            let user = resolve_user_or_me("GitLab", user, me, || {
                discover_gitlab_user(&instance, token.as_deref())
            })?;
            let window = resolve_date_window(window)?;
            let cache_root = run_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| out.clone());
            let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
            let ing = make_gitlab_ingestor(
                &user,
                window.since,
                window.until,
                &state,
                &instance,
                include_reviews,
                throttle_ms,
                token,
                cache_dir,
            )
            .context("create GitLab ingestor")?;
            let ingest = ing.ingest().context("ingest events")?;

            let window_label = window.window_label();

            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }

        Source::Jira {
            user,
            auth_user,
            window,
            status,
            instance,
            throttle_ms,
            token,
            cache_dir,
            no_cache,
        } => {
            let window = resolve_date_window(window)?;
            let cache_root = run_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| out.clone());
            let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
            let ing = make_jira_ingestor(
                &user,
                auth_user,
                window.since,
                window.until,
                &status,
                &instance,
                throttle_ms,
                token,
                cache_dir,
            )
            .context("create Jira ingestor")?;
            let ingest = ing.ingest().context("ingest events")?;

            let window_label = window.window_label();

            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }

        Source::Linear {
            user_id,
            window,
            status,
            project,
            throttle_ms,
            api_key,
            cache_dir,
            no_cache,
        } => {
            let window = resolve_date_window(window)?;
            let cache_root = run_dir
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| out.clone());
            let cache_dir = resolve_cache_dir(&cache_root, cache_dir, no_cache);
            let ing = make_linear_ingestor(
                &user_id,
                window.since,
                window.until,
                &status,
                project,
                throttle_ms,
                api_key,
                cache_dir,
            )
            .context("create Linear ingestor")?;
            let ingest = ing.ingest().context("ingest events")?;

            let window_label = window.window_label();

            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let outputs = engine
                .refresh(
                    ingest,
                    &user_id,
                    &window_label,
                    &run_dir,
                    zip,
                    &bundle_profile,
                )
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }

        Source::Json {
            events,
            coverage,
            user,
            window_label,
        } => {
            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let ing = JsonIngestor {
                events_path: events,
                coverage_path: coverage,
            };
            let ingest = ing.ingest().context("ingest events")?;

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }

        Source::Manual {
            events,
            user,
            window,
        } => {
            let window = resolve_date_window(window)?;
            if !shiplog_workstreams::WorkstreamManager::has_curated(&run_dir)
                && !shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir).exists()
            {
                anyhow::bail!(
                    "No workstreams found in {:?}. Run `shiplog collect` first.",
                    run_dir
                );
            }

            let ing = ManualIngestor::new(&events, user.clone(), window.since, window.until);
            let ingest = ing.ingest().context("ingest events")?;
            let window_label = window.window_label();

            let outputs = engine
                .refresh(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("refresh engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Refreshed while preserving workstream curation:");
            print_outputs_simple(&outputs);
        }
    }

    Ok(())
}
