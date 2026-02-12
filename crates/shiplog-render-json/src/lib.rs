use anyhow::{Context, Result};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use std::io::Write;
use std::path::Path;

/// Write canonical events to JSONL.
///
/// JSONL is the right primitive:
/// - line-delimited, append-friendly
/// - diff-friendly
/// - can be streamed
pub fn write_events_jsonl(path: &Path, events: &[EventEnvelope]) -> Result<()> {
    let mut f = std::fs::File::create(path).with_context(|| format!("create {path:?}"))?;
    for ev in events {
        let line = serde_json::to_string(ev).context("serialize event")?;
        f.write_all(line.as_bytes())?;
        f.write_all(b"\n")?;
    }
    Ok(())
}

pub fn write_coverage_manifest(path: &Path, cov: &CoverageManifest) -> Result<()> {
    let text = serde_json::to_string_pretty(cov).context("serialize coverage")?;
    std::fs::write(path, text).with_context(|| format!("write {path:?}"))?;
    Ok(())
}
