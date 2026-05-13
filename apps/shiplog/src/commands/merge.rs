use anyhow::Context;
use std::path::PathBuf;

use crate::*;

pub(super) fn handle(
    inputs: Vec<PathBuf>,
    out: PathBuf,
    conflict: MergeConflict,
    user: Option<String>,
    window_label: Option<String>,
    zip: bool,
    redact_key: Option<String>,
    bundle_profile: BundleProfile,
    regen: bool,
) -> Result<()> {
    let redaction_key = RedactionKey::resolve(redact_key, &bundle_profile)?;
    let clusterer: Box<dyn shiplog_ports::WorkstreamClusterer> = Box::new(RepoClusterer);
    let (engine, redactor) = create_engine(redaction_key.engine_key(), clusterer, &bundle_profile);
    let engine = engine.with_profile_rendering(redaction_key.render_profiles());

    let mut ingest_outputs = Vec::with_capacity(inputs.len());
    for input in &inputs {
        ingest_outputs.push(
            load_run_ingest(input)
                .with_context(|| format!("load merge input {}", input.display()))?,
        );
    }

    let merged = engine
        .merge(ingest_outputs, conflict.into())
        .context("merge input runs")?;
    let merge_user = user.unwrap_or_else(|| merged.coverage.user.clone());
    let merge_window_label = window_label.unwrap_or_else(|| {
        format!(
            "{}..{}",
            merged.coverage.window.since, merged.coverage.window.until
        )
    });
    let run_id = merged.coverage.run_id.to_string();
    let run_dir = out.join(&run_id);

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
            merged,
            &merge_user,
            &merge_window_label,
            &run_dir,
            zip,
            &bundle_profile,
        )
        .context("run merged engine pipeline")?;

    redactor
        .save_cache(&cache_path)
        .with_context(|| format!("save redaction cache to {cache_path:?}"))?;

    println!("Merged and wrote:");
    println!("- inputs: {}", inputs.len());
    println!("- conflict: {}", conflict.as_str());
    print_outputs(&outputs, ws_source);

    Ok(())
}
