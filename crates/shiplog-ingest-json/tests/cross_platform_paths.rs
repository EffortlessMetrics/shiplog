//! Cross-platform path handling tests for shiplog-ingest-json.
//!
//! Ensures JSONL file reading works correctly with paths containing spaces,
//! unicode characters, and special characters on all platforms.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, RunId};
use shiplog_ingest_json::JsonIngestor;
use shiplog_output_layout::{FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL};
use shiplog_ports::Ingestor;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use std::io::Write;
use std::path::Path;

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
    dir: &Path,
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

// ---------------------------------------------------------------------------
// Paths with spaces
// ---------------------------------------------------------------------------

#[test]
fn ingestor_reads_from_path_with_spaces() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("path with spaces");
    std::fs::create_dir_all(&dir).unwrap();

    let events = vec![make_event("org/repo", "e1"), make_event("org/repo2", "e2")];
    let (events_path, coverage_path) = write_jsonl_and_coverage(&dir, &events, &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 2);
    assert_eq!(output.coverage.user, "tester");
}

// ---------------------------------------------------------------------------
// Paths with unicode characters
// ---------------------------------------------------------------------------

#[test]
fn ingestor_reads_from_path_with_unicode() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("données_événements");
    std::fs::create_dir_all(&dir).unwrap();

    let events = vec![make_event("org/repo", "e1")];
    let (events_path, coverage_path) = write_jsonl_and_coverage(&dir, &events, &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

#[test]
fn ingestor_reads_from_path_with_cjk_chars() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("日本語テスト");
    std::fs::create_dir_all(&dir).unwrap();

    let events = vec![make_event("org/repo", "e1")];
    let (events_path, coverage_path) = write_jsonl_and_coverage(&dir, &events, &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

// ---------------------------------------------------------------------------
// Paths with special characters
// ---------------------------------------------------------------------------

#[test]
fn ingestor_reads_from_path_with_parens_and_hyphens() {
    let root = tempfile::tempdir().unwrap();
    let dir = root.path().join("project (v2.0-beta)");
    std::fs::create_dir_all(&dir).unwrap();

    let events = vec![make_event("org/repo", "e1")];
    let (events_path, coverage_path) = write_jsonl_and_coverage(&dir, &events, &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

// ---------------------------------------------------------------------------
// Deeply nested paths
// ---------------------------------------------------------------------------

#[test]
fn ingestor_reads_from_deeply_nested_path() {
    let root = tempfile::tempdir().unwrap();
    let dir = root
        .path()
        .join("a")
        .join("b")
        .join("c")
        .join("d")
        .join("events");
    std::fs::create_dir_all(&dir).unwrap();

    let events = vec![make_event("org/repo", "e1")];
    let (events_path, coverage_path) = write_jsonl_and_coverage(&dir, &events, &make_coverage());

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

// ---------------------------------------------------------------------------
// Files in different directories (events and coverage in separate dirs)
// ---------------------------------------------------------------------------

#[test]
fn ingestor_reads_files_from_separate_directories() {
    let root = tempfile::tempdir().unwrap();
    let events_dir = root.path().join("events dir");
    let coverage_dir = root.path().join("coverage dir");
    std::fs::create_dir_all(&events_dir).unwrap();
    std::fs::create_dir_all(&coverage_dir).unwrap();

    let events_path = events_dir.join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = coverage_dir.join(FILE_COVERAGE_MANIFEST_JSON);

    let ev = make_event("org/repo", "e1");
    let mut f = std::fs::File::create(&events_path).unwrap();
    writeln!(f, "{}", serde_json::to_string(&ev).unwrap()).unwrap();

    std::fs::write(
        &coverage_path,
        serde_json::to_string(&make_coverage()).unwrap(),
    )
    .unwrap();

    let ing = JsonIngestor {
        events_path,
        coverage_path,
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}

// ---------------------------------------------------------------------------
// Path constructed with Path::join (not string concatenation)
// ---------------------------------------------------------------------------

#[test]
fn path_join_produces_valid_paths_for_ingestor() {
    let root = tempfile::tempdir().unwrap();
    // Use Path::join to construct paths (the correct cross-platform approach)
    let subdir = root.path().join("sub dir");
    std::fs::create_dir_all(&subdir).unwrap();

    let events_path = subdir.join(FILE_LEDGER_EVENTS_JSONL);
    let coverage_path = subdir.join(FILE_COVERAGE_MANIFEST_JSON);

    // Verify the paths are proper children of subdir
    assert!(events_path.starts_with(&subdir));
    assert!(coverage_path.starts_with(&subdir));

    let events = vec![make_event("org/repo", "e1")];
    let mut f = std::fs::File::create(&events_path).unwrap();
    for ev in &events {
        writeln!(f, "{}", serde_json::to_string(ev).unwrap()).unwrap();
    }
    std::fs::write(
        &coverage_path,
        serde_json::to_string(&make_coverage()).unwrap(),
    )
    .unwrap();

    let ing = JsonIngestor {
        events_path: events_path.clone(),
        coverage_path: coverage_path.clone(),
    };
    let output = ing.ingest().unwrap();
    assert_eq!(output.events.len(), 1);
}
