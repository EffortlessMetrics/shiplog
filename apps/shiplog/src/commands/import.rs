use anyhow::Context;
use std::path::PathBuf;

use crate::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn handle(
    dir: PathBuf,
    out: PathBuf,
    user: String,
    window_label: String,
    redact_key: Option<String>,
    bundle_profile: BundleProfile,
    zip: bool,
    regen: bool,
    llm_cluster: bool,
    llm_api_endpoint: String,
    llm_model: String,
    llm_api_key: Option<String>,
) -> Result<()> {
    let events_path = dir.join("ledger.events.jsonl");
    let coverage_path = dir.join("coverage.manifest.json");

    if !events_path.exists() {
        anyhow::bail!(
            "No ledger.events.jsonl found in {:?}. Expected import directory.",
            dir
        );
    }
    if !coverage_path.exists() {
        anyhow::bail!(
            "No coverage.manifest.json found in {:?}. Expected import directory.",
            dir
        );
    }

    let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
    let clusterer = build_clusterer(llm_cluster, &llm_api_endpoint, &llm_model, llm_api_key);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let ingest = ing.ingest().context("ingest events")?;
    let run_id = ingest.coverage.run_id.to_string();
    let run_dir = out.join(&run_id);

    // If --regen, delete stale workstream files so the engine reclusters
    if regen {
        let curated = run_dir.join("workstreams.yaml");
        let suggested = run_dir.join("workstreams.suggested.yaml");
        let _ = std::fs::remove_file(&curated);
        let _ = std::fs::remove_file(&suggested);
    }

    // Load workstreams from import dir (unless --regen)
    let workstreams = if regen {
        None
    } else {
        shiplog_workstreams::WorkstreamManager::try_load(&dir)
            .context("load workstreams from import directory")?
    };

    let cache_path = DeterministicRedactor::cache_path(&run_dir);
    let _ = redactor.load_cache(&cache_path);

    let (outputs, ws_source) = engine
        .import(
            ingest,
            &user,
            &window_label,
            &run_dir,
            zip,
            workstreams,
            &bundle_profile,
        )
        .context("import engine pipeline")?;

    redactor
        .save_cache(&cache_path)
        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

    println!("Imported and wrote:");
    print_outputs(&outputs, ws_source);

    Ok(())
}
