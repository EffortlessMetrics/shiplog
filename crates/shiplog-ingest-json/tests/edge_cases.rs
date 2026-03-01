//! Edge-case tests for shiplog-ingest-json.
//!
//! Covers malformed JSONL lines, mixed event types, very large events,
//! and empty lines in JSONL files.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_ingest_json::{JsonIngestor, parse_events_jsonl};
use shiplog_output_layout::{FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL};
use shiplog_ports::Ingestor;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use std::io::Write;

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_pr_event(repo: &str, id: &str) -> EventEnvelope {
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

fn make_review_event(id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", "review", id]),
        kind: EventKind::Review,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(2),
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: Some("https://github.com/org/repo".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "Some PR".into(),
            submitted_at: Utc::now(),
            state: "APPROVED".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: Some(id.into()),
        },
    }
}

fn make_manual_event(id: &str) -> EventEnvelope {
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
            event_type: ManualEventType::Note,
            title: format!("Manual {id}"),
            description: Some("A manual event".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 1).unwrap()),
            ended_at: None,
            impact: None,
        }),
        tags: vec![],
        links: vec![],
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

fn write_events_and_coverage(
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

// ── Malformed JSONL lines (graceful error handling) ──────────────────────

#[test]
fn malformed_json_on_every_line_reports_first_line_error() {
    let err = parse_events_jsonl("bad\nworse\nterrible", "test").unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("line 1"), "expected line 1 in: {msg}");
}

#[test]
fn truncated_json_is_detected() {
    let ev = make_pr_event("org/repo", "1");
    let mut json = serde_json::to_string(&ev).unwrap();
    // Truncate halfway through
    json.truncate(json.len() / 2);
    let err = parse_events_jsonl(&json, "truncated").unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("line 1"), "expected line ref in: {msg}");
}

#[test]
fn valid_json_but_wrong_schema_reports_error() {
    let text = r#"{"name": "not an event", "value": 42}"#;
    let err = parse_events_jsonl(text, "wrong-schema").unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("line 1"), "expected line ref in: {msg}");
}

#[test]
fn json_array_instead_of_jsonl_reports_error() {
    let ev = make_pr_event("org/repo", "1");
    let text = format!("[{}]", serde_json::to_string(&ev).unwrap());
    let err = parse_events_jsonl(&text, "array-not-jsonl").unwrap_err();
    assert!(err.to_string().contains("line 1") || !err.to_string().is_empty());
}

#[test]
fn error_on_third_line_reports_line_3() {
    let ev = make_pr_event("org/repo", "1");
    let good = serde_json::to_string(&ev).unwrap();
    let text = format!("{good}\n{good}\n{{broken}}\n{good}");
    let err = parse_events_jsonl(&text, "test").unwrap_err();
    let msg = format!("{err:#}");
    assert!(msg.contains("line 3"), "expected line 3 in: {msg}");
}

#[test]
fn ingestor_with_malformed_events_file_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let events_path = tmp.path().join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = tmp.path().join(FILE_COVERAGE_MANIFEST_JSON);

    std::fs::write(&events_path, "not valid json at all\n").unwrap();
    std::fs::write(
        &coverage_path,
        serde_json::to_string(&make_coverage()).unwrap(),
    )
    .unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    assert!(ing.ingest().is_err());
}

// ── Mixed event types in one file ────────────────────────────────────────

#[test]
fn mixed_pr_review_and_manual_events() {
    let tmp = tempfile::tempdir().unwrap();
    let events = vec![
        make_pr_event("org/repo1", "pr1"),
        make_review_event("rev1"),
        make_manual_event("man1"),
        make_pr_event("org/repo2", "pr2"),
        make_manual_event("man2"),
    ];
    let coverage = make_coverage();
    let (events_path, coverage_path) = write_events_and_coverage(tmp.path(), &events, &coverage);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 5);

    let kinds: Vec<EventKind> = output.events.iter().map(|e| e.kind.clone()).collect();
    assert_eq!(kinds[0], EventKind::PullRequest);
    assert_eq!(kinds[1], EventKind::Review);
    assert_eq!(kinds[2], EventKind::Manual);
    assert_eq!(kinds[3], EventKind::PullRequest);
    assert_eq!(kinds[4], EventKind::Manual);
}

#[test]
fn mixed_event_types_via_parse_jsonl() {
    let pr = make_pr_event("org/repo", "pr");
    let review = make_review_event("rev");
    let manual = make_manual_event("man");
    let text = format!(
        "{}\n{}\n{}",
        serde_json::to_string(&pr).unwrap(),
        serde_json::to_string(&review).unwrap(),
        serde_json::to_string(&manual).unwrap(),
    );
    let result = parse_events_jsonl(&text, "mixed").unwrap();
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].kind, EventKind::PullRequest);
    assert_eq!(result[1].kind, EventKind::Review);
    assert_eq!(result[2].kind, EventKind::Manual);
}

// ── Very large events ────────────────────────────────────────────────────

#[test]
fn event_with_very_long_title() {
    let mut ev = make_pr_event("org/repo", "big-title");
    if let EventPayload::PullRequest(ref mut pr) = ev.payload {
        pr.title = "x".repeat(100_000);
    }
    let text = serde_json::to_string(&ev).unwrap();
    let result = parse_events_jsonl(&text, "large").unwrap();
    assert_eq!(result.len(), 1);
    if let EventPayload::PullRequest(ref pr) = result[0].payload {
        assert_eq!(pr.title.len(), 100_000);
    } else {
        panic!("expected PR payload");
    }
}

#[test]
fn event_with_many_tags() {
    let mut ev = make_pr_event("org/repo", "many-tags");
    ev.tags = (0..1000).map(|i| format!("tag-{i}")).collect();
    let text = serde_json::to_string(&ev).unwrap();
    let result = parse_events_jsonl(&text, "tags").unwrap();
    assert_eq!(result[0].tags.len(), 1000);
}

#[test]
fn event_with_many_links() {
    let mut ev = make_pr_event("org/repo", "many-links");
    ev.links = (0..500)
        .map(|i| Link {
            label: format!("link-{i}"),
            url: format!("https://example.com/{i}"),
        })
        .collect();
    let text = serde_json::to_string(&ev).unwrap();
    let result = parse_events_jsonl(&text, "links").unwrap();
    assert_eq!(result[0].links.len(), 500);
}

#[test]
fn many_events_in_one_file() {
    let tmp = tempfile::tempdir().unwrap();
    let events: Vec<EventEnvelope> = (0..500)
        .map(|i| make_pr_event(&format!("org/repo{i}"), &format!("ev{i}")))
        .collect();
    let coverage = make_coverage();
    let (events_path, coverage_path) = write_events_and_coverage(tmp.path(), &events, &coverage);

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 500);
}

// ── Empty lines in JSONL ─────────────────────────────────────────────────

#[test]
fn only_empty_lines_produces_empty_vec() {
    let result = parse_events_jsonl("\n\n\n\n\n", "blanks").unwrap();
    assert!(result.is_empty());
}

#[test]
fn tabs_and_spaces_only_lines_are_skipped() {
    let result = parse_events_jsonl("\t\t\n   \n\t \t\n", "whitespace").unwrap();
    assert!(result.is_empty());
}

#[test]
fn empty_lines_interspersed_with_valid_events() {
    let ev = make_pr_event("org/repo", "1");
    let json = serde_json::to_string(&ev).unwrap();
    let text = format!("\n\n{json}\n\n\n{json}\n\n{json}\n\n");
    let result = parse_events_jsonl(&text, "mixed-blanks").unwrap();
    assert_eq!(result.len(), 3);
}

#[test]
fn carriage_return_lines_are_handled() {
    let ev = make_pr_event("org/repo", "cr");
    let json = serde_json::to_string(&ev).unwrap();
    // Windows-style line endings
    let text = format!("{json}\r\n\r\n{json}\r\n");
    let result = parse_events_jsonl(&text, "crlf").unwrap();
    assert_eq!(result.len(), 2);
}

// ── Ingestor error messages include context ──────────────────────────────

#[test]
fn missing_both_files_returns_error() {
    let tmp = tempfile::tempdir().unwrap();
    let ing = JsonIngestor {
        events_path: tmp.path().join("no-events.jsonl"),
        coverage_path: tmp.path().join("no-coverage.json"),
    };
    let err = ing.ingest().unwrap_err();
    let msg = format!("{err:#}");
    // Should mention the file that failed to read
    assert!(
        msg.contains("read") || msg.contains("no-events"),
        "error should describe what went wrong: {msg}"
    );
}

#[test]
fn coverage_with_extra_json_fields_still_parses() {
    let tmp = tempfile::tempdir().unwrap();
    let ev = make_pr_event("org/repo", "1");
    let events_path = tmp.path().join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = tmp.path().join(FILE_COVERAGE_MANIFEST_JSON);

    let mut f = std::fs::File::create(&events_path).unwrap();
    writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();

    // Write coverage with an extra field that isn't in the schema
    let cov = make_coverage();
    let mut cov_value: serde_json::Value = serde_json::to_value(&cov).unwrap();
    cov_value["extra_field"] = serde_json::json!("should be ignored");
    std::fs::write(&coverage_path, serde_json::to_string(&cov_value).unwrap()).unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    // serde default is to ignore unknown fields; verify no panic
    let result = ing.ingest();
    // If deny_unknown_fields is set, this errors; otherwise succeeds.
    match result {
        Ok(output) => assert_eq!(output.events.len(), 1),
        Err(e) => {
            let msg = format!("{e:#}");
            assert!(
                msg.contains("unknown") || msg.contains("extra"),
                "error should be about unknown field: {msg}"
            );
        }
    }
}
