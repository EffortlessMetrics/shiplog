//! JSONL/JSON ingestion adapter for prebuilt shiplog ledgers.
//!
//! Loads canonical `ledger.events.jsonl` + `coverage.manifest.json` and returns
//! them through the [`shiplog_ports::Ingestor`] interface.

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

/// Parse JSONL text into a vector of [`EventEnvelope`]s.
///
/// Each non-empty line is parsed as a JSON-encoded `EventEnvelope`.
/// `source` is included in error context messages.
pub fn parse_events_jsonl(text: &str, source: &str) -> Result<Vec<EventEnvelope>> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let ev: EventEnvelope = serde_json::from_str(line)
            .with_context(|| format!("parse event json line {} in {source}", i + 1))?;
        out.push(ev);
    }
    Ok(out)
}

fn read_events(path: &PathBuf) -> Result<Vec<EventEnvelope>> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    parse_events_jsonl(&text, &format!("{path:?}"))
}

fn read_coverage(path: &PathBuf) -> Result<CoverageManifest> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {path:?}"))?;
    let cov: CoverageManifest = serde_json::from_str(&text).with_context(|| "parse coverage")?;
    Ok(cov)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, Utc};
    use shiplog_ids::{EventId, RunId};
    use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
    use shiplog_schema::event::*;
    use std::io::Write;

    fn make_test_event(repo_name: &str, event_id: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", event_id]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "testuser".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo_name.into(),
                html_url: Some(format!("https://github.com/{repo_name}")),
                visibility: RepoVisibility::Public,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: "Test PR".into(),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(2),
                changed_files: Some(3),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![],
            source: SourceRef {
                system: SourceSystem::JsonImport,
                url: None,
                opaque_id: None,
            },
        }
    }

    fn make_test_coverage() -> CoverageManifest {
        CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "testuser".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["json-import".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        }
    }

    #[test]
    fn valid_jsonl_roundtrip() {
        let temp = tempfile::tempdir().unwrap();
        let events_path = temp.path().join("ledger.events.jsonl");
        let coverage_path = temp.path().join("coverage.manifest.json");

        let ev1 = make_test_event("org/repo1", "ev1");
        let ev2 = make_test_event("org/repo2", "ev2");
        let coverage = make_test_coverage();

        // Write events as JSONL
        {
            let mut f = std::fs::File::create(&events_path).unwrap();
            writeln!(f, "{}", serde_json::to_string(&ev1).unwrap()).unwrap();
            writeln!(f, "{}", serde_json::to_string(&ev2).unwrap()).unwrap();
        }
        std::fs::write(&coverage_path, serde_json::to_string(&coverage).unwrap()).unwrap();

        let ing = JsonIngestor {
            events_path,
            coverage_path,
        };
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 2);
        assert_eq!(output.events[0].repo.full_name, "org/repo1");
        assert_eq!(output.events[1].repo.full_name, "org/repo2");
        assert_eq!(output.coverage.user, "testuser");
    }

    #[test]
    fn missing_events_file_returns_error() {
        let temp = tempfile::tempdir().unwrap();
        let events_path = temp.path().join("nonexistent.jsonl");
        let coverage_path = temp.path().join("coverage.manifest.json");

        let coverage = make_test_coverage();
        std::fs::write(&coverage_path, serde_json::to_string(&coverage).unwrap()).unwrap();

        let ing = JsonIngestor {
            events_path,
            coverage_path,
        };
        let result = ing.ingest();
        assert!(result.is_err());
    }

    #[test]
    fn blank_lines_in_jsonl_are_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let events_path = temp.path().join("ledger.events.jsonl");
        let coverage_path = temp.path().join("coverage.manifest.json");

        let ev = make_test_event("org/repo", "ev1");
        let coverage = make_test_coverage();

        // Write with blank lines
        {
            let mut f = std::fs::File::create(&events_path).unwrap();
            writeln!(f).unwrap();
            writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();
            writeln!(f).unwrap();
            writeln!(f, "   ").unwrap();
        }
        std::fs::write(&coverage_path, serde_json::to_string(&coverage).unwrap()).unwrap();

        let ing = JsonIngestor {
            events_path,
            coverage_path,
        };
        let output = ing.ingest().unwrap();
        assert_eq!(output.events.len(), 1);
    }

    #[test]
    fn invalid_json_line_returns_error_with_line_number() {
        let temp = tempfile::tempdir().unwrap();
        let events_path = temp.path().join("ledger.events.jsonl");
        let coverage_path = temp.path().join("coverage.manifest.json");

        let ev = make_test_event("org/repo", "ev1");
        let coverage = make_test_coverage();

        {
            let mut f = std::fs::File::create(&events_path).unwrap();
            writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();
            writeln!(f, "{{not valid json}}").unwrap();
        }
        std::fs::write(&coverage_path, serde_json::to_string(&coverage).unwrap()).unwrap();

        let ing = JsonIngestor {
            events_path,
            coverage_path,
        };
        let result = ing.ingest();
        assert!(result.is_err());
        let err_msg = format!("{:#}", result.unwrap_err());
        assert!(
            err_msg.contains("line 2"),
            "Expected error to mention line number, got: {err_msg}"
        );
    }
}
