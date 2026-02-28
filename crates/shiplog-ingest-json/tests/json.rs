//! Comprehensive tests for shiplog-ingest-json: JSONL parsing, error handling, edge cases.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_ingest_json::{JsonIngestor, parse_events_jsonl};
use shiplog_output_layout::{FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL};
use shiplog_ports::Ingestor;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use std::io::Write;

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_event(repo: &str, id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "tester".into(),
            id: Some(1),
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
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
            changed_files: Some(1),
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

fn make_manual_event(id: &str, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["manual", id]),
        kind: EventKind::Manual,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "tester".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "manual/events".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Other,
            title: title.into(),
            description: Some("A manual event".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: None,
            impact: None,
        }),
        tags: vec!["manual".into()],
        links: vec![Link {
            label: "doc".into(),
            url: "https://example.com".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: Some(id.into()),
        },
    }
}

fn make_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc::now(),
        user: "tester".into(),
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

fn write_jsonl_and_coverage(
    dir: &std::path::Path,
    events: &[EventEnvelope],
    coverage: &CoverageManifest,
) -> (std::path::PathBuf, std::path::PathBuf) {
    let events_path = dir.join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = dir.join(FILE_COVERAGE_MANIFEST_JSON);

    let mut f = std::fs::File::create(&events_path).unwrap();
    for ev in events {
        writeln!(f, "{}", serde_json::to_string(ev).unwrap()).unwrap();
    }

    std::fs::write(&coverage_path, serde_json::to_string(coverage).unwrap()).unwrap();

    (events_path, coverage_path)
}

// ── parse_events_jsonl ──────────────────────────────────────────────────────

#[test]
fn parse_empty_string_returns_empty_vec() {
    let result = parse_events_jsonl("", "test").unwrap();
    assert!(result.is_empty());
}

#[test]
fn parse_whitespace_only_returns_empty_vec() {
    let result = parse_events_jsonl("  \n  \n  ", "test").unwrap();
    assert!(result.is_empty());
}

#[test]
fn parse_single_event() {
    let ev = make_event("org/repo", "1");
    let line = serde_json::to_string(&ev).unwrap();
    let result = parse_events_jsonl(&line, "test").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].repo.full_name, "org/repo");
}

#[test]
fn parse_multiple_events() {
    let ev1 = make_event("org/repo1", "1");
    let ev2 = make_event("org/repo2", "2");
    let ev3 = make_event("org/repo3", "3");
    let text = format!(
        "{}\n{}\n{}",
        serde_json::to_string(&ev1).unwrap(),
        serde_json::to_string(&ev2).unwrap(),
        serde_json::to_string(&ev3).unwrap(),
    );
    let result = parse_events_jsonl(&text, "test").unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].repo.full_name, "org/repo1");
    assert_eq!(result[2].repo.full_name, "org/repo3");
}

#[test]
fn parse_skips_blank_lines_between_events() {
    let ev = make_event("org/repo", "1");
    let line = serde_json::to_string(&ev).unwrap();
    let text = format!("\n\n{}\n  \n\n{}\n\n", line, line);
    let result = parse_events_jsonl(&text, "test").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn parse_invalid_json_reports_line_number() {
    let ev = make_event("org/repo", "1");
    let line = serde_json::to_string(&ev).unwrap();
    let text = format!("{}\n{{bad json}}\n{}", line, line);
    let err = parse_events_jsonl(&text, "myfile.jsonl").unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("line 2"), "Expected line 2 in: {msg}");
    assert!(
        msg.contains("myfile.jsonl"),
        "Expected source name in: {msg}"
    );
}

#[test]
fn parse_invalid_json_on_first_line() {
    let err = parse_events_jsonl("not json at all", "src").unwrap_err();
    let msg = format!("{:#}", err);
    assert!(msg.contains("line 1"), "Expected line 1 in: {msg}");
}

#[test]
fn parse_mixed_event_types() {
    let pr_event = make_event("org/repo", "pr1");
    let manual_event = make_manual_event("m1", "Design review");
    let text = format!(
        "{}\n{}",
        serde_json::to_string(&pr_event).unwrap(),
        serde_json::to_string(&manual_event).unwrap(),
    );
    let result = parse_events_jsonl(&text, "test").unwrap();
    assert_eq!(result.len(), 2);
    assert_eq!(result[0].kind, EventKind::PullRequest);
    assert_eq!(result[1].kind, EventKind::Manual);
}

#[test]
fn parse_trailing_newline() {
    let ev = make_event("org/repo", "1");
    let text = format!("{}\n", serde_json::to_string(&ev).unwrap());
    let result = parse_events_jsonl(&text, "test").unwrap();
    assert_eq!(result.len(), 1);
}

#[test]
fn parse_preserves_event_order() {
    let events: Vec<_> = (0..10)
        .map(|i| make_event(&format!("org/repo{i}"), &format!("ev{i}")))
        .collect();
    let text: String = events
        .iter()
        .map(|e| serde_json::to_string(e).unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let result = parse_events_jsonl(&text, "test").unwrap();
    assert_eq!(result.len(), 10);
    for (i, ev) in result.iter().enumerate() {
        assert_eq!(ev.repo.full_name, format!("org/repo{i}"));
    }
}

// ── JsonIngestor (file-based) ───────────────────────────────────────────────

#[test]
fn ingestor_roundtrip() {
    let tmp = tempfile::tempdir().unwrap();
    let events = vec![make_event("org/r1", "e1"), make_event("org/r2", "e2")];
    let coverage = make_coverage();
    let (events_path, coverage_path) = write_jsonl_and_coverage(tmp.path(), &events, &coverage);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 2);
    assert_eq!(output.coverage.user, "tester");
    assert_eq!(output.coverage.mode, "merged");
}

#[test]
fn ingestor_empty_events_file() {
    let tmp = tempfile::tempdir().unwrap();
    let (events_path, coverage_path) = write_jsonl_and_coverage(tmp.path(), &[], &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert!(output.events.is_empty());
}

#[test]
fn ingestor_missing_events_file() {
    let tmp = tempfile::tempdir().unwrap();
    let coverage_path = tmp.path().join(FILE_COVERAGE_MANIFEST_JSON);
    std::fs::write(
        &coverage_path,
        serde_json::to_string(&make_coverage()).unwrap(),
    )
    .unwrap();

    let ing = JsonIngestor {
        events_path: tmp.path().join("missing.jsonl"),
        coverage_path,
    };
    assert!(ing.ingest().is_err());
}

#[test]
fn ingestor_missing_coverage_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ev = make_event("org/r", "e1");
    let events_path = tmp.path().join(FILE_LEDGER_EVENTS_JSONL);
    let mut f = std::fs::File::create(&events_path).unwrap();
    writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path: tmp.path().join("missing.json"),
    };
    assert!(ing.ingest().is_err());
}

#[test]
fn ingestor_invalid_coverage_json() {
    let tmp = tempfile::tempdir().unwrap();
    let ev = make_event("org/r", "e1");
    let events_path = tmp.path().join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = tmp.path().join(FILE_COVERAGE_MANIFEST_JSON);

    let mut f = std::fs::File::create(&events_path).unwrap();
    writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();
    std::fs::write(&coverage_path, "{not valid}").unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    assert!(ing.ingest().is_err());
}

#[test]
fn ingestor_with_manual_events() {
    let tmp = tempfile::tempdir().unwrap();
    let events = vec![
        make_manual_event("m1", "Design doc"),
        make_manual_event("m2", "Architecture review"),
    ];
    let coverage = make_coverage();
    let (events_path, coverage_path) = write_jsonl_and_coverage(tmp.path(), &events, &coverage);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 2);
    assert!(output.events.iter().all(|e| e.kind == EventKind::Manual));
}

#[test]
fn ingestor_preserves_all_event_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let ev = make_manual_event("x", "Important thing");
    let coverage = make_coverage();
    let (events_path, coverage_path) = write_jsonl_and_coverage(tmp.path(), &[ev], &coverage);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    let loaded = &output.events[0];
    assert_eq!(loaded.tags, vec!["manual".to_string()]);
    assert_eq!(loaded.links.len(), 1);
    assert_eq!(loaded.links[0].label, "doc");
    assert_eq!(loaded.source.system, SourceSystem::Manual);
    assert_eq!(loaded.source.opaque_id.as_deref(), Some("x"));
}

#[test]
fn ingestor_coverage_fields_preserved() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cov = make_coverage();
    cov.warnings = vec!["partial data".into()];
    cov.completeness = Completeness::Partial;
    cov.sources = vec!["github".into(), "manual".into()];

    let (events_path, coverage_path) = write_jsonl_and_coverage(tmp.path(), &[], &cov);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.coverage.completeness, Completeness::Partial);
    assert_eq!(output.coverage.warnings, vec!["partial data".to_string()]);
    assert_eq!(output.coverage.sources.len(), 2);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod prop {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn parse_blank_lines_never_produce_events(
            blanks in proptest::collection::vec("[ \\t]*", 0..20)
        ) {
            let text = blanks.join("\n");
            let result = parse_events_jsonl(&text, "prop").unwrap();
            prop_assert!(result.is_empty());
        }

        #[test]
        fn event_count_matches_input(count in 1usize..10) {
            let events: Vec<_> = (0..count)
                .map(|i| make_event("org/r", &format!("e{i}")))
                .collect();
            let text: String = events
                .iter()
                .map(|e| serde_json::to_string(e).unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            let result = parse_events_jsonl(&text, "prop").unwrap();
            prop_assert_eq!(result.len(), count);
        }
    }
}
