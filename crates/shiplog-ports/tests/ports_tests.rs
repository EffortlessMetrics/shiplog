//! Tests for shiplog-ports crate.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::{IngestOutput, Ingestor, Redactor, Renderer, WorkstreamClusterer};
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::sync::Arc;

/// Mock ingestor that returns empty results.
struct MockIngestor;

impl Ingestor for MockIngestor {
    fn ingest(&self) -> anyhow::Result<IngestOutput> {
        Ok(IngestOutput {
            events: vec![],
            coverage: CoverageManifest {
                run_id: RunId::now("test"),
                generated_at: Utc::now(),
                user: "test".to_string(),
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                },
                mode: "test".to_string(),
                sources: vec![],
                slices: vec![],
                warnings: vec![],
                completeness: Completeness::Complete,
            },
        })
    }
}

/// Mock clusterer that returns empty workstreams.
struct MockClusterer;

impl WorkstreamClusterer for MockClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: events
                .iter()
                .map(|e| Workstream {
                    id: WorkstreamId::from_parts(["ws", &e.id.0]),
                    title: format!("Workstream {}", e.id.0),
                    summary: None,
                    tags: vec![],
                    events: vec![e.id.clone()],
                    receipts: vec![],
                    stats: WorkstreamStats::zero(),
                })
                .collect(),
        })
    }
}

/// Mock renderer that returns a simple string.
struct MockRenderer;

impl Renderer for MockRenderer {
    fn render_packet_markdown(
        &self,
        user: &str,
        window_label: &str,
        events: &[EventEnvelope],
        _workstreams: &WorkstreamsFile,
        _coverage: &CoverageManifest,
    ) -> anyhow::Result<String> {
        Ok(format!(
            "User: {}, Window: {}, Events: {}",
            user,
            window_label,
            events.len()
        ))
    }
}

/// Mock redactor that returns events unchanged.
struct MockRedactor;

impl Redactor for MockRedactor {
    fn redact_events(
        &self,
        events: &[EventEnvelope],
        _profile: &str,
    ) -> anyhow::Result<Vec<EventEnvelope>> {
        Ok(events.to_vec())
    }

    fn redact_workstreams(
        &self,
        workstreams: &WorkstreamsFile,
        _profile: &str,
    ) -> anyhow::Result<WorkstreamsFile> {
        Ok(workstreams.clone())
    }
}

fn create_test_pr_event(id: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["pr", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
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
            number,
            title: title.to_string(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
            additions: Some(100),
            deletions: Some(50),
            changed_files: Some(5),
            touched_paths_hint: vec!["src/main.rs".to_string()],
            window: None,
        }),
        tags: vec!["test".to_string()],
        links: vec![Link {
            label: "pr".to_string(),
            url: format!("https://github.com/owner/repo/pull/{}", number),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com/".to_string()),
            opaque_id: None,
        },
    }
}

#[test]
fn mock_ingestor_returns_valid_output() {
    let ingestor = MockIngestor;
    let output = ingestor.ingest().unwrap();
    // Should return valid empty output with coverage manifest
    assert!(output.events.is_empty());
    assert!(!output.coverage.user.is_empty());
}

#[test]
fn mock_clusterer_groups_events() {
    let clusterer = MockClusterer;
    let event = create_test_pr_event("1", 1, "Test PR");

    let result = clusterer.cluster(&[event]).unwrap();
    assert_eq!(result.workstreams.len(), 1);
}

#[test]
fn mock_clusterer_empty_events_returns_empty() {
    let clusterer = MockClusterer;
    let result = clusterer.cluster(&[]).unwrap();
    assert!(result.workstreams.is_empty());
}

#[test]
fn mock_renderer_produces_output() {
    let renderer = MockRenderer;
    let event = create_test_pr_event("1", 1, "Test PR");

    let result = renderer
        .render_packet_markdown(
            "testuser",
            "2024-W01",
            &[event],
            &WorkstreamsFile {
                version: 1,
                generated_at: Utc::now(),
                workstreams: vec![],
            },
            &CoverageManifest {
                run_id: RunId::now("test"),
                generated_at: Utc::now(),
                user: "test".to_string(),
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                },
                mode: "test".to_string(),
                sources: vec![],
                slices: vec![],
                warnings: vec![],
                completeness: Completeness::Complete,
            },
        )
        .unwrap();

    assert!(result.contains("testuser"));
    assert!(result.contains("2024-W01"));
    assert!(result.contains("1"));
}

#[test]
fn mock_redactor_preserves_events() {
    let redactor = MockRedactor;
    let event = create_test_pr_event("1", 1, "Test");

    let result = redactor.redact_events(&[event], "public").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].tags, vec!["test".to_string()]);
}

#[test]
fn mock_redactor_preserves_workstreams() {
    let redactor = MockRedactor;
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "1"]),
            title: "Test Workstream".to_string(),
            summary: None,
            tags: vec!["secret".to_string()],
            events: vec![],
            receipts: vec![],
            stats: WorkstreamStats::zero(),
        }],
    };

    let result = redactor.redact_workstreams(&workstreams, "public").unwrap();
    assert_eq!(result.workstreams.len(), 1);
    // WorkstreamId is deterministically hashed from parts
    assert_eq!(result.workstreams[0].id.0.len(), 64); // SHA-256 hex is 64 chars
}

/// Test trait objects can be used through Arc<dyn>
#[test]
fn trait_objects_work_with_arc() {
    let ingestor: Arc<dyn Ingestor> = Arc::new(MockIngestor);
    let output = ingestor.ingest().unwrap();
    assert!(output.events.is_empty());

    let clusterer: Arc<dyn WorkstreamClusterer> = Arc::new(MockClusterer);
    let result = clusterer.cluster(&[]).unwrap();
    assert!(result.workstreams.is_empty());

    let renderer: Arc<dyn Renderer> = Arc::new(MockRenderer);
    let result = renderer
        .render_packet_markdown(
            "user",
            "label",
            &[],
            &WorkstreamsFile {
                version: 1,
                generated_at: Utc::now(),
                workstreams: vec![],
            },
            &CoverageManifest {
                run_id: RunId::now("test"),
                generated_at: Utc::now(),
                user: "test".to_string(),
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2024, 1, 2).unwrap(),
                },
                mode: "test".to_string(),
                sources: vec![],
                slices: vec![],
                warnings: vec![],
                completeness: Completeness::Complete,
            },
        )
        .unwrap();
    assert!(result.contains("user"));

    let redactor: Arc<dyn Redactor> = Arc::new(MockRedactor);
    let result = redactor.redact_events(&[], "public").unwrap();
    assert!(result.is_empty());
}

/// Test IngestOutput struct can be constructed with various data
#[test]
fn ingest_output_with_events() {
    let event = create_test_pr_event("test123", 42, "Fix bug");

    let output = IngestOutput {
        events: vec![event.clone()],
        coverage: CoverageManifest {
            run_id: RunId::now("test"),
            generated_at: Utc::now(),
            user: "tester".to_string(),
            window: TimeWindow {
                since: NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                until: NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
            },
            mode: "merged".to_string(),
            sources: vec!["github".to_string()],
            slices: vec![],
            warnings: vec![],
            completeness: Completeness::Complete,
        },
    };

    assert_eq!(output.events.len(), 1);
    assert_eq!(output.events[0].id.0, event.id.0);
}

/// Test clusterer produces valid workstreams
#[test]
fn clusterer_produces_valid_workstreams() {
    let clusterer = MockClusterer;
    let events = vec![
        create_test_pr_event("1", 1, "PR 1"),
        create_test_pr_event("2", 2, "PR 2"),
    ];

    let result = clusterer.cluster(&events).unwrap();

    // Each event should produce one workstream
    assert_eq!(result.workstreams.len(), 2);

    // Workstreams should have valid structure
    for ws in &result.workstreams {
        assert!(!ws.id.0.is_empty());
        assert!(!ws.title.is_empty());
    }
}
