use anyhow::Context;
use std::path::PathBuf;

use crate::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle(
    source: CollectSource,
    out: Option<PathBuf>,
    zip: bool,
    redact_key: Option<String>,
    bundle_profile: Option<BundleProfile>,
    regen: bool,
    llm_cluster: bool,
    llm_api_endpoint: String,
    llm_model: String,
    llm_api_key: Option<String>,
) -> Result<()> {
    let source = match source {
        CollectSource::Multi {
            config,
            window,
            conflict,
        } => {
            let config_model = load_shiplog_config(&config)?;
            ensure_supported_config_version(&config_model)?;
            let base_dir = config_base_dir(&config);
            let out = out
                .clone()
                .unwrap_or_else(|| config_default_out(&config_model, &base_dir));
            let bundle_profile = bundle_profile.clone().map(Ok).unwrap_or_else(|| {
                doctor_config_profile(config_model.defaults.profile.as_deref())
            })?;
            let redaction_key = RedactionKey::resolve_with_env(
                redact_key.clone(),
                &bundle_profile,
                &config_redaction_key_env(&config_model),
            )?;
            let clusterer = build_clusterer(
                llm_cluster,
                &llm_api_endpoint,
                &llm_model,
                llm_api_key.clone(),
            );
            let (engine, redactor) =
                create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
            let engine = engine.with_profile_rendering(redaction_key.render_profiles());
            let window = resolve_multi_window(window, &config_model)?;
            let configured =
                collect_configured_sources(&config, &config_model, window.clone(), &out)?;
            let result = run_configured_multi_pipeline(
                &config,
                &config_model,
                &out,
                window,
                &bundle_profile,
                conflict,
                configured,
                regen,
                false,
                zip,
                &engine,
                redactor,
            )?;

            println!("Collected configured sources:");
            for (name, ingest) in &result.configured.successes {
                println!(
                    "- {name}: success, {}",
                    event_count_phrase(ingest.events.len())
                );
            }
            for failure in &result.configured.failures {
                println!("- {}: skipped, {}", failure.name, failure.error);
            }
            println!("Merged and wrote:");
            println!("- inputs: {}", result.configured.successes.len());
            println!("- conflict: {}", conflict.as_str());
            print_outputs(&result.outputs, result.ws_source);
            return Ok(());
        }

        CollectSource::Source(source) => source,
    };
    let out = out.unwrap_or_else(|| PathBuf::from("./out"));
    let bundle_profile = bundle_profile.unwrap_or_default();
    let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
    let clusterer = build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());

    match source {
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
            let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
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
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
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
            let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
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
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
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
            let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
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
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
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
            let cache_dir = resolve_cache_dir(&out, cache_dir, no_cache);
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
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(
                    ingest,
                    &user_id,
                    &window_label,
                    &run_dir,
                    zip,
                    &bundle_profile,
                )
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
        }

        Source::Json {
            events,
            coverage,
            user,
            window_label,
        } => {
            let ing = JsonIngestor {
                events_path: events,
                coverage_path: coverage,
            };
            let ingest = ing.ingest().context("ingest events")?;
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
        }

        Source::Manual {
            events,
            user,
            window,
        } => {
            let window = resolve_date_window(window)?;
            let ing = ManualIngestor::new(&events, user.clone(), window.since, window.until);
            let ingest = ing.ingest().context("ingest events")?;
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);
            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
        }

        Source::Git {
            repo,
            window,
            author,
            include_merges,
        } => {
            let window = resolve_date_window(window)?;
            let ing = make_git_ingestor(&repo, window.since, window.until, author, include_merges);
            let ingest = ing.ingest().context("ingest events")?;
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);
            let window_label = window.window_label();

            // Check if user has curated workstreams and warn
            if !regen && shiplog_workstreams::WorkstreamManager::has_curated(&run_dir) {
                eprintln!("Note: Using existing workstreams.yaml (user-curated).");
                eprintln!("      Use --regen to regenerate suggestions.");
            }

            // If --regen, delete existing suggested workstreams so the engine regenerates them
            if regen {
                let suggested = shiplog_workstreams::WorkstreamManager::suggested_path(&run_dir);
                if suggested.exists() {
                    std::fs::remove_file(&suggested)
                        .with_context(|| format!("remove {:?} for --regen", suggested))?;
                }
            }

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(
                    ingest,
                    "local",
                    &window_label,
                    &run_dir,
                    zip,
                    &bundle_profile,
                )
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Collected and wrote:");
            print_outputs(&outputs, ws_source);
        }
    }

    Ok(())
}
