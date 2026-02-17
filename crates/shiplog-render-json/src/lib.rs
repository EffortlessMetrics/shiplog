//! JSON writers for canonical shiplog artifacts.
//!
//! Emits line-delimited events (`ledger.events.jsonl`) and pretty-formatted
//! coverage manifests (`coverage.manifest.json`).

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone, Utc};
    use shiplog_ids::EventId;
    use shiplog_ids::RunId;
    use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
    use shiplog_schema::event::*;

    fn pr_event(repo: &str, number: u64, title: &str) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
            kind: EventKind::PullRequest,
            occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo.to_string(),
                html_url: Some(format!("https://github.com/{repo}")),
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number,
                title: title.to_string(),
                state: PullRequestState::Merged,
                created_at: Utc.timestamp_opt(0, 0).unwrap(),
                merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
                additions: Some(1),
                deletions: Some(0),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: Some(TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                }),
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".into(),
                url: format!("https://github.com/{repo}/pull/{number}"),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: Some("https://api.github.com/...".into()),
                opaque_id: None,
            },
        }
    }

    fn test_coverage() -> CoverageManifest {
        CoverageManifest {
            run_id: RunId("test_run".into()),
            generated_at: Utc.timestamp_opt(0, 0).unwrap(),
            user: "tester".into(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        }
    }

    #[test]
    fn jsonl_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let events = vec![
            pr_event("acme/foo", 1, "First PR"),
            pr_event("acme/foo", 2, "Second PR"),
        ];

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        for (i, line) in text.lines().enumerate() {
            let ev: EventEnvelope = serde_json::from_str(line)
                .unwrap_or_else(|e| panic!("line {i} failed to parse: {e}"));
            assert_eq!(ev.id, events[i].id);
        }
    }

    #[test]
    fn coverage_manifest_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("coverage.manifest.json");

        let cov = test_coverage();
        write_coverage_manifest(&path, &cov).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();
        assert_eq!(loaded.run_id, cov.run_id);
        assert_eq!(loaded.user, cov.user);
        assert_eq!(loaded.completeness, cov.completeness);
    }

    #[test]
    fn empty_events_produces_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        write_events_jsonl(&path, &[]).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.is_empty());
    }

    #[test]
    fn multiple_events_one_per_line() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");

        let n = 5;
        let events: Vec<_> = (1..=n)
            .map(|i| pr_event("acme/foo", i, &format!("PR {i}")))
            .collect();

        write_events_jsonl(&path, &events).unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), n as usize);
    }
}
