use anyhow::{Context, Result};
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use std::path::PathBuf;

/// Simple adapter that ingests JSONL events + a JSON coverage manifest.
///
/// This is useful for:
/// - tests
/// - fixtures
/// - future "org mode" where an upstream collector produces a ledger and shiplog just renders
pub struct JsonIngestor {
    pub events_path: PathBuf,
    pub coverage_path: PathBuf,
}

impl Ingestor for JsonIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        let events = read_events(&self.events_path)?;
        let coverage = read_coverage(&self.coverage_path)?;
        Ok(IngestOutput { events, coverage })
    }
}

fn read_events(path: &PathBuf) -> Result<Vec<EventEnvelope>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: EventEnvelope = serde_json::from_str(line)
            .with_context(|| format!("parse event json line {} in {:?}", i + 1, path))?;
        out.push(ev);
    }
    Ok(out)
}

fn read_coverage(path: &PathBuf) -> Result<CoverageManifest> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let cov: CoverageManifest = serde_json::from_str(&text).with_context(|| "parse coverage")?;
    Ok(cov)
}
