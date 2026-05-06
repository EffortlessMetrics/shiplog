//! Format stability tests for shiplog-render-md.
//!
//! Complements `snapshot.rs` with deterministic Markdown packet snapshots
//! covering all event types, section ordering variants, and edge cases
//! to guard against accidental format changes in the rendered packet.

use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::Renderer;
use shiplog_render_md::{MarkdownRenderer, SectionOrder};
use shiplog_schema::{
    coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow},
    event::*,
    workstream::{Workstream, WorkstreamStats, WorkstreamsFile},
};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 1, 12, 0, 0).unwrap()
}

fn window() -> TimeWindow {
    TimeWindow {
        since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
        until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }
}

fn deterministic_coverage(completeness: Completeness) -> CoverageManifest {
    CoverageManifest {
        run_id: RunId("stability_test_run".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: window(),
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness,
    }
}

fn pr_envelope(repo: &str, number: u64, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "pr", repo, &number.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: ts(),
        actor: Actor {
            login: "octocat".into(),
            id: Some(1),
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number,
            title: title.into(),
            state: PullRequestState::Merged,
            created_at: ts(),
            merged_at: Some(ts()),
            additions: Some(50),
            deletions: Some(10),
            changed_files: Some(3),
            touched_paths_hint: vec!["src/lib.rs".into()],
            window: Some(window()),
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: Some(format!(
                "https://api.github.com/repos/{repo}/pulls/{number}"
            )),
            opaque_id: None,
        },
    }
}

fn review_envelope(repo: &str, pull_number: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["github", "review", repo, &pull_number.to_string(), "1"]),
        kind: EventKind::Review,
        occurred_at: ts(),
        actor: Actor {
            login: "reviewer".into(),
            id: Some(2),
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number,
            pull_title: format!("PR #{pull_number}"),
            submitted_at: ts(),
            state: "approved".into(),
            window: Some(window()),
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: format!("https://github.com/{repo}/pull/{pull_number}"),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn manual_envelope(title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["manual", "acme/widgets", &title.replace(' ', "-")]),
        kind: EventKind::Manual,
        occurred_at: ts(),
        actor: Actor {
            login: "octocat".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/widgets".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: title.into(),
            description: Some("Detailed description of the event".into()),
            started_at: Some(NaiveDate::from_ymd_opt(2025, 3, 15).unwrap()),
            ended_at: Some(NaiveDate::from_ymd_opt(2025, 3, 16).unwrap()),
            impact: Some("Reduced MTTR by 50%".into()),
        }),
        tags: vec!["incident".into()],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_workstreams(workstreams: Vec<Workstream>) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: ts(),
        workstreams,
    }
}

// ============================================================================
// Full packet with all event types (PR + Review + Manual)
// ============================================================================

#[test]
fn format_stability_packet_all_event_types() {
    let pr = pr_envelope("acme/widgets", 42, "Implement feature X");
    let review = review_envelope("acme/widgets", 42);
    let manual = manual_envelope("Production outage resolution");

    let events = vec![pr.clone(), review.clone(), manual.clone()];
    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "acme-widgets"]),
        title: "Widgets Core".into(),
        summary: Some("Core widget development and incident response".into()),
        tags: vec!["core".into()],
        stats: WorkstreamStats {
            pull_requests: 1,
            reviews: 1,
            manual_events: 1,
        },
        events: vec![pr.id.clone(), review.id.clone(), manual.id.clone()],
        receipts: vec![pr.id.clone(), review.id.clone(), manual.id.clone()],
    };

    let workstreams = make_workstreams(vec![ws]);
    let coverage = deterministic_coverage(Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("octocat", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!("packet_all_event_types", result);
}

// ============================================================================
// Packet with multiple workstreams
// ============================================================================

#[test]
fn format_stability_packet_multiple_workstreams() {
    let pr1 = pr_envelope("acme/widgets", 10, "Add widget caching");
    let pr2 = pr_envelope("acme/widgets", 11, "Fix widget rendering");
    let pr3 = pr_envelope("acme/infra", 5, "Setup CI pipeline");
    let pr4 = pr_envelope("acme/infra", 6, "Add monitoring dashboards");

    let events = vec![pr1.clone(), pr2.clone(), pr3.clone(), pr4.clone()];

    let ws1 = Workstream {
        id: WorkstreamId::from_parts(["repo", "acme/widgets"]),
        title: "Widgets Core".into(),
        summary: Some("Widget improvements".into()),
        tags: vec!["core".into(), "frontend".into()],
        stats: WorkstreamStats {
            pull_requests: 2,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![pr1.id.clone(), pr2.id.clone()],
        receipts: vec![pr1.id.clone(), pr2.id.clone()],
    };
    let ws2 = Workstream {
        id: WorkstreamId::from_parts(["repo", "acme/infra"]),
        title: "Infrastructure".into(),
        summary: Some("CI/CD and observability".into()),
        tags: vec!["infra".into(), "devops".into()],
        stats: WorkstreamStats {
            pull_requests: 2,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![pr3.id.clone(), pr4.id.clone()],
        receipts: vec![pr3.id.clone(), pr4.id.clone()],
    };

    let workstreams = make_workstreams(vec![ws1, ws2]);
    let coverage = deterministic_coverage(Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("octocat", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!("packet_multiple_workstreams", result);
}

// ============================================================================
// Packet with CoverageFirst section ordering
// ============================================================================

#[test]
fn format_stability_packet_coverage_first() {
    let pr = pr_envelope("acme/widgets", 1, "Initial commit");
    let events = vec![pr.clone()];

    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "init"]),
        title: "Initial Work".into(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats {
            pull_requests: 1,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![pr.id.clone()],
        receipts: vec![pr.id.clone()],
    };

    let workstreams = make_workstreams(vec![ws]);
    let coverage = deterministic_coverage(Completeness::Complete);

    let result = MarkdownRenderer::new()
        .with_section_order(SectionOrder::CoverageFirst)
        .render_packet_markdown("octocat", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!("packet_coverage_first_ordering", result);
}

// ============================================================================
// Packet with partial coverage and warnings
// ============================================================================

#[test]
fn format_stability_packet_partial_coverage() {
    let pr = pr_envelope("acme/widgets", 1, "Some work");
    let events = vec![pr.clone()];

    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "partial"]),
        title: "Partial Work".into(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats {
            pull_requests: 1,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![pr.id.clone()],
        receipts: vec![pr.id.clone()],
    };

    let workstreams = make_workstreams(vec![ws]);
    let coverage = CoverageManifest {
        run_id: RunId("stability_test_run".into()),
        generated_at: ts(),
        user: "octocat".into(),
        window: window(),
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr author:octocat merged:2025-01-01..2025-02-01".into(),
                total_count: 200,
                fetched: 100,
                incomplete_results: Some(true),
                notes: vec![],
            },
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
                },
                query: "is:pr author:octocat merged:2025-02-01..2025-03-01".into(),
                total_count: 30,
                fetched: 30,
                incomplete_results: Some(false),
                notes: vec![],
            },
        ],
        warnings: vec![
            "API rate limit hit during ingestion".into(),
            "Some repositories may be missing".into(),
        ],
        completeness: Completeness::Partial,
    };

    let result = MarkdownRenderer::new()
        .render_packet_markdown("octocat", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!("packet_partial_coverage", result);
}

// ============================================================================
// Empty packet (no events, no workstreams)
// ============================================================================

#[test]
fn format_stability_packet_empty() {
    let workstreams = make_workstreams(vec![]);
    let coverage = deterministic_coverage(Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("octocat", "2025-Q1", &[], &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!("packet_empty", result);
}
