use anyhow::Context;
use std::path::PathBuf;

use crate::*;

#[expect(clippy::too_many_arguments, reason = "policy:clippy-0001")]
pub(super) fn handle(
    source: Source,
    out: PathBuf,
    zip: bool,
    redact_key: Option<String>,
    bundle_profile: BundleProfile,
    llm_cluster: bool,
    llm_api_endpoint: String,
    llm_model: String,
    llm_api_key: Option<String>,
) -> Result<()> {
    // Legacy mode: just do collect
    let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
    let clusterer = build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());

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
            let run_id = ingest.coverage.run_id.to_string();
            let run_dir = out.join(&run_id);

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let window_label = window.window_label();
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

            println!("Wrote:");
            print_outputs(&outputs, ws_source);
        }
        Source::Github {
            user,
            me,
            window,
            mode,
            repo_owners,
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
                repo_owners,
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let window_label = window.window_label();
            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Wrote:");
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let window_label = window.window_label();
            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Wrote:");
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let window_label = window.window_label();
            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Wrote:");
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let window_label = window.window_label();
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

            println!("Wrote:");
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Wrote:");
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

            let cache_path = DeterministicRedactor::cache_path(&run_dir);
            let _ = redactor.load_cache(&cache_path);

            let (outputs, ws_source) = engine
                .run(ingest, &user, &window_label, &run_dir, zip, &bundle_profile)
                .context("run engine pipeline")?;

            redactor
                .save_cache(&cache_path)
                .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

            println!("Wrote:");
            print_outputs(&outputs, ws_source);
        }
    }

    Ok(())
}
