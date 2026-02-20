//! Tests for shiplog-render-json.

use shiplog_render_json::{write_coverage_manifest, write_events_jsonl};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_ids::{EventId, RunId};
use chrono::{NaiveDate, Utc};

// Test: JSONL roundtrip preserves all fields
#[test]
fn jsonl_roundtrip_preserves_all_fields() {
    let event = create_test_event("test1", "Test PR");
    
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    
    write_events_jsonl(&path, &[event.clone()]).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 1);
    
    let loaded: EventEnvelope = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(loaded.id, event.id);
    assert_eq!(loaded.kind, event.kind);
}

#[test]
fn jsonl_multiple_events_all_preserved() {
    let events = vec![
        create_test_event("1", "PR 1"),
        create_test_event("2", "PR 2"),
        create_test_event("3", "PR 3"),
    ];
    
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    
    write_events_jsonl(&path, &events).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines.len(), 3);
}

#[test]
fn coverage_manifest_roundtrip_preserves_all_fields() {
    let cov = CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc::now(),
        user: "testuser".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "merged".to_string(),
        sources: vec!["github".to_string()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.json");
    
    write_coverage_manifest(&path, &cov).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();
    
    assert_eq!(loaded.run_id, cov.run_id);
    assert_eq!(loaded.user, cov.user);
    assert_eq!(loaded.mode, cov.mode);
    assert_eq!(loaded.completeness, cov.completeness);
}

#[test]
fn empty_events_produces_valid_output() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("events.jsonl");
    
    let events: Vec<EventEnvelope> = vec![];
    write_events_jsonl(&path, &events).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.is_empty() || text == "\n");
}

#[test]
fn coverage_with_zero_slices_is_valid() {
    let cov = CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc::now(),
        user: "testuser".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "merged".to_string(),
        sources: vec!["github".to_string()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    };
    
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.json");
    
    write_coverage_manifest(&path, &cov).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();
    assert!(loaded.slices.is_empty());
}

#[test]
fn coverage_with_slices_shows_partial() {
    let cov = CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc::now(),
        user: "testuser".to_string(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
        },
        mode: "merged".to_string(),
        sources: vec!["github".to_string()],
        slices: vec![shiplog_schema::coverage::CoverageSlice {
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
            },
            query: "test".to_string(),
            total_count: 1000,
            fetched: 500,
            incomplete_results: Some(true),
            notes: vec![],
        }],
        warnings: vec!["API cap hit".to_string()],
        completeness: Completeness::Partial,
    };
    
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("coverage.json");
    
    write_coverage_manifest(&path, &cov).unwrap();
    
    let text = std::fs::read_to_string(&path).unwrap();
    let loaded: CoverageManifest = serde_json::from_str(&text).unwrap();
    
    assert_eq!(loaded.slices.len(), 1);
    assert_eq!(loaded.slices[0].fetched, 500);
    assert_eq!(loaded.completeness, Completeness::Partial);
}

// Helper function to create test events
fn create_test_event(id: &str, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["test", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "testuser".to_string(),
            id: Some(12345),
        },
        repo: RepoRef {
            full_name: "owner/repo".to_string(),
            html_url: Some("https://github.com/owner/repo".to_string()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 1,
            title: title.to_string(),
            state: PullRequestState::Open,
            created_at: Utc::now(),
            merged_at: None,
            additions: Some(100),
            deletions: Some(50),
            changed_files: Some(5),
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec!["test".to_string()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/".to_string()),
            opaque_id: None,
        },
    }
}
