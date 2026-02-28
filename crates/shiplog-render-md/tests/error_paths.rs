//! Error-path and edge-case tests for the Markdown renderer.
//!
//! Exercises empty workstreams, missing receipts, large event counts,
//! partial coverage with warnings, and workstream-event mismatches.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::Renderer;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow};
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, RepoVisibility, SourceRef, SourceSystem,
};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn pr_event(id: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["errtest", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "dev".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: Some("https://github.com/acme/repo".into()),
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: Utc.timestamp_opt(0, 0).unwrap(),
            merged_at: Some(Utc.timestamp_opt(0, 0).unwrap()),
            additions: Some(1),
            deletions: Some(0),
            changed_files: Some(1),
            touched_paths_hint: vec![],
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/acme/repo/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: Some(id.into()),
        },
    }
}

fn base_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("render_error_test".into()),
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

// ---------------------------------------------------------------------------
// Empty workstreams with non-empty events
// ---------------------------------------------------------------------------

#[test]
fn render_empty_workstreams_with_events_succeeds() {
    let renderer = MarkdownRenderer::new();
    let events = vec![pr_event("1", 1, "Feature A")];
    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![],
    };

    let result = renderer.render_packet_markdown(
        "dev",
        "2025-01-01..2025-02-01",
        &events,
        &ws,
        &base_coverage(),
    );

    let md = result.unwrap();
    assert!(
        md.contains("No workstreams found"),
        "should note no workstreams"
    );
    assert!(md.contains("1 PRs"), "should still count events");
}

#[test]
fn render_completely_empty_inputs_succeeds() {
    let renderer = MarkdownRenderer::new();
    let result = renderer.render_packet_markdown(
        "nobody",
        "2025-01-01..2025-02-01",
        &[],
        &WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![],
        },
        &base_coverage(),
    );

    let md = result.unwrap();
    assert!(md.contains("# Summary"), "should have summary section");
    assert!(md.contains("0 PRs"), "PR count should be 0");
}

// ---------------------------------------------------------------------------
// Workstream references non-existent events (orphan receipts)
// ---------------------------------------------------------------------------

#[test]
fn render_workstream_with_orphan_receipts_does_not_panic() {
    let renderer = MarkdownRenderer::new();
    let events = vec![pr_event("1", 1, "Real PR")];

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "orphan"]),
            title: "Orphan Work".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: 3,
                reviews: 0,
                manual_events: 0,
            },
            events: vec![
                EventId::from_parts(["errtest", "1"]),
                EventId::from_parts(["missing", "99"]),
                EventId::from_parts(["missing", "100"]),
            ],
            receipts: vec![
                EventId::from_parts(["errtest", "1"]),
                EventId::from_parts(["missing", "99"]),
            ],
        }],
    };

    let result = renderer.render_packet_markdown(
        "dev",
        "2025-01-01..2025-02-01",
        &events,
        &ws,
        &base_coverage(),
    );

    // Should not error; orphan IDs are silently skipped
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Many workstreams
// ---------------------------------------------------------------------------

#[test]
fn render_many_workstreams_succeeds() {
    let renderer = MarkdownRenderer::new();
    let events: Vec<EventEnvelope> = (0..50)
        .map(|i| pr_event(&i.to_string(), i as u64, &format!("PR #{i}")))
        .collect();

    let workstreams: Vec<Workstream> = (0..20)
        .map(|i| {
            let event_ids: Vec<EventId> = events
                .iter()
                .skip(i * 2)
                .take(2)
                .map(|e| e.id.clone())
                .collect();
            Workstream {
                id: WorkstreamId::from_parts(["ws", &format!("stream-{i}")]),
                title: format!("Workstream {i}"),
                summary: Some(format!("Summary for stream {i}")),
                tags: vec![],
                stats: WorkstreamStats {
                    pull_requests: event_ids.len(),
                    reviews: 0,
                    manual_events: 0,
                },
                events: event_ids.clone(),
                receipts: event_ids,
            }
        })
        .collect();

    let ws_file = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams,
    };

    let result = renderer.render_packet_markdown(
        "dev",
        "2025-01-01..2025-02-01",
        &events,
        &ws_file,
        &base_coverage(),
    );

    let md = result.unwrap();
    assert!(md.contains("Workstream 0"));
    assert!(md.contains("Workstream 19"));
    assert!(md.contains("**Workstreams:** 20"));
}

// ---------------------------------------------------------------------------
// Coverage with warnings and partial slices
// ---------------------------------------------------------------------------

#[test]
fn render_partial_coverage_with_many_warnings() {
    let renderer = MarkdownRenderer::new();
    let events = vec![pr_event("1", 1, "Feature")];

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "partial"]),
            title: "Partial Work".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: 1,
                reviews: 0,
                manual_events: 0,
            },
            events: vec![events[0].id.clone()],
            receipts: vec![events[0].id.clone()],
        }],
    };

    let coverage = CoverageManifest {
        warnings: vec![
            "Rate limit hit during fetch".into(),
            "Some PRs may be missing".into(),
            "Review data incomplete".into(),
        ],
        completeness: Completeness::Partial,
        slices: vec![
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                },
                query: "is:pr author:dev merged:2025-01-01..2025-01-14".into(),
                total_count: 1500,
                fetched: 1000,
                incomplete_results: Some(true),
                notes: vec![],
            },
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 15).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr author:dev merged:2025-01-15..2025-01-31".into(),
                total_count: 50,
                fetched: 50,
                incomplete_results: Some(false),
                notes: vec![],
            },
        ],
        ..base_coverage()
    };

    let result =
        renderer.render_packet_markdown("dev", "2025-01-01..2025-02-01", &events, &ws, &coverage);

    let md = result.unwrap();
    assert!(md.contains("Partial"), "should show partial completeness");
    assert!(
        md.contains("Rate limit hit"),
        "warnings should appear in output"
    );
    assert!(
        md.contains("incomplete results"),
        "incomplete slices should be noted"
    );
}

// ---------------------------------------------------------------------------
// Coverage with no sources
// ---------------------------------------------------------------------------

#[test]
fn render_coverage_with_empty_sources() {
    let renderer = MarkdownRenderer::new();
    let coverage = CoverageManifest {
        sources: vec![],
        ..base_coverage()
    };

    let result = renderer.render_packet_markdown(
        "dev",
        "2025-01-01..2025-02-01",
        &[],
        &WorkstreamsFile {
            version: 1,
            generated_at: Utc::now(),
            workstreams: vec![],
        },
        &coverage,
    );

    assert!(result.is_ok());
    let md = result.unwrap();
    assert!(md.contains("**Sources:**"), "sources header should exist");
}

// ---------------------------------------------------------------------------
// Large event counts
// ---------------------------------------------------------------------------

#[test]
fn render_large_event_list_succeeds() {
    let renderer = MarkdownRenderer::new();

    let events: Vec<EventEnvelope> = (0..500)
        .map(|i| pr_event(&i.to_string(), i as u64, &format!("Large PR #{i}")))
        .collect();

    let event_ids: Vec<EventId> = events.iter().map(|e| e.id.clone()).collect();

    let ws = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["ws", "large"]),
            title: "Large Workstream".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: 500,
                reviews: 0,
                manual_events: 0,
            },
            events: event_ids.clone(),
            receipts: event_ids,
        }],
    };

    let result = renderer.render_packet_markdown(
        "dev",
        "2025-01-01..2025-02-01",
        &events,
        &ws,
        &base_coverage(),
    );

    let md = result.unwrap();
    assert!(md.contains("500 PRs"), "should count all 500 events");
    assert!(
        md.contains("more in [Appendix]"),
        "should truncate receipts with appendix link"
    );
}
