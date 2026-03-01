//! Additional comprehensive tests for shiplog-ports: error paths, edge cases, composition.

use anyhow::anyhow;
use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::{IngestOutput, Ingestor, Redactor, Renderer, WorkstreamClusterer};
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, ManualEvent, ManualEventType,
    PullRequestEvent, PullRequestState, RepoRef, RepoVisibility, ReviewEvent, SourceRef,
    SourceSystem,
};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ── Helpers ─────────────────────────────────────────────────────────────────

fn make_coverage(user: &str) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc::now(),
        user: user.into(),
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

fn make_workstreams(count: usize) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: (0..count)
            .map(|i| Workstream {
                id: WorkstreamId::from_parts(["ws", &i.to_string()]),
                title: format!("Workstream {i}"),
                summary: Some(format!("Summary for ws {i}")),
                tags: vec!["test".into()],
                events: vec![EventId::from_parts(["ev", &i.to_string()])],
                receipts: vec![],
                stats: WorkstreamStats::zero(),
            })
            .collect(),
    }
}

fn make_pr_event(id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["pr", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        actor: Actor {
            login: "octocat".into(),
            id: Some(42),
        },
        repo: RepoRef {
            full_name: "owner/repo".into(),
            html_url: Some("https://github.com/owner/repo".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 10,
            title: "Fix bug".into(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(1_700_000_000, 0).unwrap()),
            additions: Some(50),
            deletions: Some(20),
            changed_files: Some(3),
            touched_paths_hint: vec!["src/lib.rs".into()],
            window: None,
        }),
        tags: vec!["bugfix".into()],
        links: vec![Link {
            label: "PR".into(),
            url: "https://github.com/owner/repo/pull/10".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some("https://api.github.com".into()),
            opaque_id: Some("pr-10".into()),
        },
    }
}

fn make_manual_event(id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["manual", id]),
        kind: EventKind::Manual,
        occurred_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        actor: Actor {
            login: "octocat".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "manual/events".into(),
            html_url: None,
            visibility: RepoVisibility::Private,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Other,
            title: "Design review".into(),
            description: Some("Reviewed architecture".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 1, 16).unwrap()),
            impact: Some("High".into()),
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_review_event(id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["review", id]),
        kind: EventKind::Review,
        occurred_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(99),
        },
        repo: RepoRef {
            full_name: "org/project".into(),
            html_url: Some("https://github.com/org/project".into()),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 5,
            pull_title: "Add feature".into(),
            state: "APPROVED".into(),
            submitted_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            window: None,
        }),
        tags: vec!["review".into()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

// ── Error-returning implementations ─────────────────────────────────────────

struct FailingIngestor {
    message: String,
}

impl Ingestor for FailingIngestor {
    fn ingest(&self) -> anyhow::Result<IngestOutput> {
        Err(anyhow!("{}", self.message))
    }
}

struct FailingClusterer;

impl WorkstreamClusterer for FailingClusterer {
    fn cluster(&self, _events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
        Err(anyhow!("clustering failed"))
    }
}

struct FailingRenderer;

impl Renderer for FailingRenderer {
    fn render_packet_markdown(
        &self,
        _user: &str,
        _window_label: &str,
        _events: &[EventEnvelope],
        _workstreams: &WorkstreamsFile,
        _coverage: &CoverageManifest,
    ) -> anyhow::Result<String> {
        Err(anyhow!("render failed"))
    }
}

struct FailingRedactor;

impl Redactor for FailingRedactor {
    fn redact_events(
        &self,
        _events: &[EventEnvelope],
        _profile: &str,
    ) -> anyhow::Result<Vec<EventEnvelope>> {
        Err(anyhow!("redaction failed"))
    }

    fn redact_workstreams(
        &self,
        _workstreams: &WorkstreamsFile,
        _profile: &str,
    ) -> anyhow::Result<WorkstreamsFile> {
        Err(anyhow!("workstream redaction failed"))
    }
}

// ── Filtering redactor ──────────────────────────────────────────────────────

struct FilterRedactor;

impl Redactor for FilterRedactor {
    fn redact_events(
        &self,
        events: &[EventEnvelope],
        profile: &str,
    ) -> anyhow::Result<Vec<EventEnvelope>> {
        if profile == "public" {
            // Strip private repos from public profile
            Ok(events
                .iter()
                .filter(|e| e.repo.visibility == RepoVisibility::Public)
                .cloned()
                .collect())
        } else {
            Ok(events.to_vec())
        }
    }

    fn redact_workstreams(
        &self,
        ws: &WorkstreamsFile,
        profile: &str,
    ) -> anyhow::Result<WorkstreamsFile> {
        if profile == "public" {
            // Strip summaries
            let mut result = ws.clone();
            for w in &mut result.workstreams {
                w.summary = None;
            }
            Ok(result)
        } else {
            Ok(ws.clone())
        }
    }
}

// ── Counting clusterer ──────────────────────────────────────────────────────

struct CountingClusterer;

impl WorkstreamClusterer for CountingClusterer {
    fn cluster(&self, events: &[EventEnvelope]) -> anyhow::Result<WorkstreamsFile> {
        // Group all events into a single workstream
        Ok(WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: if events.is_empty() {
                vec![]
            } else {
                vec![Workstream {
                    id: WorkstreamId::from_parts(["all", "events"]),
                    title: format!("All {} events", events.len()),
                    summary: None,
                    tags: vec![],
                    events: events.iter().map(|e| e.id.clone()).collect(),
                    receipts: vec![],
                    stats: WorkstreamStats::zero(),
                }]
            },
        })
    }
}

// ── Tests: failing implementations ──────────────────────────────────────────

#[test]
fn failing_ingestor_returns_error() {
    let ing = FailingIngestor {
        message: "network timeout".into(),
    };
    let err = ing.ingest().unwrap_err();
    assert!(err.to_string().contains("network timeout"));
}

#[test]
fn failing_clusterer_returns_error() {
    let c = FailingClusterer;
    let err = c.cluster(&[make_pr_event("1")]).unwrap_err();
    assert!(err.to_string().contains("clustering failed"));
}

#[test]
fn failing_renderer_returns_error() {
    let r = FailingRenderer;
    let err = r
        .render_packet_markdown(
            "user",
            "window",
            &[],
            &make_workstreams(0),
            &make_coverage("user"),
        )
        .unwrap_err();
    assert!(err.to_string().contains("render failed"));
}

#[test]
fn failing_redactor_events_returns_error() {
    let r = FailingRedactor;
    let err = r.redact_events(&[], "public").unwrap_err();
    assert!(err.to_string().contains("redaction failed"));
}

#[test]
fn failing_redactor_workstreams_returns_error() {
    let r = FailingRedactor;
    let err = r
        .redact_workstreams(&make_workstreams(1), "public")
        .unwrap_err();
    assert!(err.to_string().contains("workstream redaction failed"));
}

// ── Tests: filtering redactor ───────────────────────────────────────────────

#[test]
fn filter_redactor_public_removes_private_repos() {
    let redactor = FilterRedactor;
    let events = vec![make_pr_event("1"), make_manual_event("2")]; // manual is private
    let result = redactor.redact_events(&events, "public").unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].repo.visibility, RepoVisibility::Public);
}

#[test]
fn filter_redactor_internal_keeps_all() {
    let redactor = FilterRedactor;
    let events = vec![make_pr_event("1"), make_manual_event("2")];
    let result = redactor.redact_events(&events, "internal").unwrap();
    assert_eq!(result.len(), 2);
}

#[test]
fn filter_redactor_public_strips_summaries() {
    let redactor = FilterRedactor;
    let ws = make_workstreams(2);
    let result = redactor.redact_workstreams(&ws, "public").unwrap();
    assert!(result.workstreams.iter().all(|w| w.summary.is_none()));
}

#[test]
fn filter_redactor_internal_keeps_summaries() {
    let redactor = FilterRedactor;
    let ws = make_workstreams(2);
    let result = redactor.redact_workstreams(&ws, "internal").unwrap();
    assert!(result.workstreams.iter().all(|w| w.summary.is_some()));
}

// ── Tests: counting clusterer ───────────────────────────────────────────────

#[test]
fn counting_clusterer_groups_all_events() {
    let c = CountingClusterer;
    let events = vec![
        make_pr_event("1"),
        make_pr_event("2"),
        make_manual_event("3"),
    ];
    let result = c.cluster(&events).unwrap();
    assert_eq!(result.workstreams.len(), 1);
    assert_eq!(result.workstreams[0].events.len(), 3);
    assert!(result.workstreams[0].title.contains("3"));
}

#[test]
fn counting_clusterer_empty_input() {
    let c = CountingClusterer;
    let result = c.cluster(&[]).unwrap();
    assert!(result.workstreams.is_empty());
}

// ── Tests: mixed event types ────────────────────────────────────────────────

#[test]
fn clusterer_handles_all_event_kinds() {
    let c = CountingClusterer;
    let events = vec![
        make_pr_event("pr1"),
        make_manual_event("m1"),
        make_review_event("r1"),
    ];
    let result = c.cluster(&events).unwrap();
    assert_eq!(result.workstreams[0].events.len(), 3);
}

// ── Tests: IngestOutput ─────────────────────────────────────────────────────

#[test]
fn ingest_output_clone() {
    let output = IngestOutput {
        events: vec![make_pr_event("1")],
        coverage: make_coverage("user"),
    };
    let cloned = output.clone();
    assert_eq!(cloned.events.len(), 1);
    assert_eq!(cloned.coverage.user, "user");
}

#[test]
fn ingest_output_debug() {
    let output = IngestOutput {
        events: vec![],
        coverage: make_coverage("debug_user"),
    };
    let debug = format!("{:?}", output);
    assert!(debug.contains("IngestOutput"));
    assert!(debug.contains("debug_user"));
}

#[test]
fn ingest_output_with_coverage_slices() {
    let mut cov = make_coverage("user");
    cov.slices.push(CoverageSlice {
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
        },
        query: "test query".into(),
        total_count: 50,
        fetched: 50,
        incomplete_results: Some(false),
        notes: vec!["note".into()],
    });
    cov.completeness = Completeness::Complete;

    let output = IngestOutput {
        events: vec![],
        coverage: cov,
    };
    assert_eq!(output.coverage.slices.len(), 1);
    assert_eq!(output.coverage.slices[0].total_count, 50);
}

#[test]
fn ingest_output_partial_completeness() {
    let mut cov = make_coverage("user");
    cov.completeness = Completeness::Partial;
    cov.warnings.push("data truncated".into());

    let output = IngestOutput {
        events: vec![],
        coverage: cov,
    };
    assert_eq!(output.coverage.completeness, Completeness::Partial);
    assert_eq!(output.coverage.warnings.len(), 1);
}

// ── Tests: trait objects with Box<dyn> ──────────────────────────────────────

#[test]
fn trait_objects_with_box() {
    let ingestor: Box<dyn Ingestor> = Box::new(FailingIngestor {
        message: "test".into(),
    });
    assert!(ingestor.ingest().is_err());

    let clusterer: Box<dyn WorkstreamClusterer> = Box::new(CountingClusterer);
    let result = clusterer.cluster(&[make_pr_event("1")]).unwrap();
    assert_eq!(result.workstreams.len(), 1);
}

// ── Tests: pipeline composition ─────────────────────────────────────────────

#[test]
fn pipeline_ingest_cluster_render_redact() {
    // Simulate a full pipeline
    struct SimpleIngestor;
    impl Ingestor for SimpleIngestor {
        fn ingest(&self) -> anyhow::Result<IngestOutput> {
            Ok(IngestOutput {
                events: vec![make_pr_event("p1"), make_review_event("r1")],
                coverage: make_coverage("octocat"),
            })
        }
    }

    struct SimpleRenderer;
    impl Renderer for SimpleRenderer {
        fn render_packet_markdown(
            &self,
            user: &str,
            _window: &str,
            events: &[EventEnvelope],
            ws: &WorkstreamsFile,
            _cov: &CoverageManifest,
        ) -> anyhow::Result<String> {
            Ok(format!(
                "# {user}\n{} events, {} workstreams",
                events.len(),
                ws.workstreams.len()
            ))
        }
    }

    let ingestor = SimpleIngestor;
    let clusterer = CountingClusterer;
    let renderer = SimpleRenderer;
    let redactor = FilterRedactor;

    let output = ingestor.ingest().unwrap();
    let workstreams = clusterer.cluster(&output.events).unwrap();
    let markdown = renderer
        .render_packet_markdown(
            &output.coverage.user,
            "2025-W01",
            &output.events,
            &workstreams,
            &output.coverage,
        )
        .unwrap();
    let redacted_events = redactor.redact_events(&output.events, "public").unwrap();

    assert!(markdown.contains("octocat"));
    assert!(markdown.contains("2 events"));
    assert!(markdown.contains("1 workstreams"));
    // Public filter keeps only public repos
    assert_eq!(redacted_events.len(), 2); // both are public
}

// ── Tests: large event collections ──────────────────────────────────────────

#[test]
fn clusterer_handles_many_events() {
    let c = CountingClusterer;
    let events: Vec<_> = (0..100).map(|i| make_pr_event(&i.to_string())).collect();
    let result = c.cluster(&events).unwrap();
    assert_eq!(result.workstreams.len(), 1);
    assert_eq!(result.workstreams[0].events.len(), 100);
}

#[test]
fn redactor_handles_many_events() {
    let r = FilterRedactor;
    let events: Vec<_> = (0..50).map(|i| make_pr_event(&i.to_string())).collect();
    let result = r.redact_events(&events, "public").unwrap();
    assert_eq!(result.len(), 50); // all public
}
