//! JSONL/JSON ingestion adapter for prebuilt shiplog ledgers.

use anyhow::{Context, Result};
use chrono::Utc;
use shiplog_ports::{IngestOutput, Ingestor};
use shiplog_schema::coverage::CoverageManifest;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::freshness::{FreshnessStatus, SourceFreshness};
use std::fmt;
use std::path::{Path, PathBuf};

/// Error returned when a JSONL event line fails to parse.
#[derive(Debug)]
pub struct JsonlParseError {
    line: usize,
    source: String,
    source_error: serde_json::Error,
}

impl JsonlParseError {
    /// The 1-based line number that failed.
    pub fn line(&self) -> usize {
        self.line
    }

    /// The caller-provided input source label.
    pub fn input_source(&self) -> &str {
        &self.source
    }
}

impl fmt::Display for JsonlParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "parse event json line {} in {}: {}",
            self.line, self.source, self.source_error
        )
    }
}

impl std::error::Error for JsonlParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source_error)
    }
}

/// Simple adapter that ingests JSONL events plus a JSON coverage manifest.
pub struct JsonIngestor {
    pub events_path: PathBuf,
    pub coverage_path: PathBuf,
}

impl Ingestor for JsonIngestor {
    fn ingest(&self) -> Result<IngestOutput> {
        let events = read_events(&self.events_path)?;
        let coverage = read_coverage(&self.coverage_path)?;
        let freshness = vec![SourceFreshness {
            source: "json_import".to_string(),
            status: FreshnessStatus::Fresh,
            cache_hits: 0,
            cache_misses: 0,
            fetched_at: Some(Utc::now()),
            reason: None,
        }];
        Ok(IngestOutput {
            events,
            coverage,
            freshness,
        })
    }
}

/// Parse newline-delimited JSON event envelopes.
///
/// Blank lines are ignored. `source` is included in parse error context so
/// callers can report the file, fixture, or input surface that failed.
pub fn parse_events_jsonl(text: &str, source: &str) -> Result<Vec<EventEnvelope>, JsonlParseError> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: EventEnvelope = serde_json::from_str(line).map_err(|err| JsonlParseError {
            line: i + 1,
            source: source.to_string(),
            source_error: err,
        })?;
        out.push(ev);
    }
    Ok(out)
}

fn read_events(path: &Path) -> Result<Vec<EventEnvelope>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    parse_events_jsonl(&text, &format!("{path:?}"))
        .with_context(|| format!("parse events jsonl {path:?}"))
}

fn read_coverage(path: &Path) -> Result<CoverageManifest> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let cov: CoverageManifest =
        serde_json::from_str(&text).with_context(|| format!("parse coverage manifest {path:?}"))?;
    Ok(cov)
}
