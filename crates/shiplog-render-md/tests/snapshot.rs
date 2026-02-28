use chrono::{NaiveDate, TimeZone, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::Renderer;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::{
    coverage::{Completeness, CoverageManifest, CoverageSlice, TimeWindow},
    event::*,
    workstream::{Workstream, WorkstreamStats, WorkstreamsFile},
};
use shiplog_testkit::{
    fixtures::{test_coverage, WorkstreamFixture},
    pr_event,
};

fn empty_workstreams() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![],
    }
}

// ── Snapshot: empty event list ──────────────────────────────────────────

#[test]
fn snapshot_empty_events() {
    let renderer = MarkdownRenderer::new();
    let coverage = test_coverage("testuser", Completeness::Complete);
    let ws = empty_workstreams();

    let result = renderer
        .render_packet_markdown("testuser", "2025-Q1", &[], &ws, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: events with special characters in titles ──────────────────

#[test]
fn snapshot_special_characters_in_titles() {
    let events = vec![
        pr_event("owner/repo", 1, "Fix <script>alert('xss')</script> injection"),
        pr_event("owner/repo", 2, "Handle `backtick` & \"quotes\" in titles"),
        pr_event("owner/repo", 3, "Support émojis 🎉 and ünïcödé"),
        pr_event("owner/repo", 4, "Pipes | and [brackets] and (parens)"),
    ];

    let ws_a = WorkstreamFixture::new("Special Characters")
        .with_summary("Workstream with tricky titles")
        .with_event(&events[0])
        .with_event(&events[1])
        .with_receipt(&events[0])
        .with_receipt(&events[1])
        .build();
    let ws_b = WorkstreamFixture::new("Unicode & Emoji")
        .with_event(&events[2])
        .with_event(&events[3])
        .with_receipt(&events[2])
        .with_receipt(&events[3])
        .build();

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws_a, ws_b],
    };
    let coverage = test_coverage("testuser", Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: redacted / stripped content ────────────────────────────────

#[test]
fn snapshot_redacted_titles() {
    let events = vec![
        pr_event("owner/repo", 10, "[REDACTED]"),
        pr_event("owner/repo", 11, "[REDACTED]"),
    ];

    let ws = WorkstreamFixture::new("[REDACTED]")
        .with_summary("[REDACTED]")
        .with_event(&events[0])
        .with_event(&events[1])
        .with_receipt(&events[0])
        .with_receipt(&events[1])
        .build();

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = test_coverage("testuser", Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: single workstream, many receipts (truncation) ─────────────

#[test]
fn snapshot_receipt_truncation() {
    let events: Vec<_> = (0..8)
        .map(|i| pr_event("owner/repo", i + 1, &format!("PR #{}", i + 1)))
        .collect();

    let mut ws_builder = WorkstreamFixture::new("Big Workstream")
        .with_summary("Lots of receipts");
    for ev in &events {
        ws_builder = ws_builder.with_event(ev).with_receipt(ev);
    }
    let ws = ws_builder.build();

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = test_coverage("testuser", Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: coverage with warnings and partial completeness ───────────

#[test]
fn snapshot_coverage_warnings_and_partial() {
    let events = vec![pr_event("owner/repo", 1, "First PR")];

    let ws = WorkstreamFixture::new("Partial Work")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = CoverageManifest {
        run_id: RunId::now("test"),
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        user: "testuser".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![
            CoverageSlice {
                window: TimeWindow {
                    since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
                    until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
                },
                query: "is:pr author:testuser merged:2025-01-01..2025-02-01".into(),
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
                query: "is:pr author:testuser merged:2025-02-01..2025-03-01".into(),
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
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: mixed event types (PR + Review + Manual) ──────────────────

#[test]
fn snapshot_mixed_event_types() {
    let pr = pr_event("owner/repo", 42, "Implement feature X");
    let review = EventEnvelope {
        id: EventId::from_parts(["review", "snap-1"]),
        kind: EventKind::Review,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "owner/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 42,
            pull_title: "Implement feature X".into(),
            submitted_at: Utc.timestamp_opt(0, 0).unwrap(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: "https://github.com/owner/repo/pull/42".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };
    let manual = EventEnvelope {
        id: EventId::from_parts(["manual", "snap-1"]),
        kind: EventKind::Manual,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "testuser".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "owner/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Incident,
            title: "Production outage resolution".into(),
            description: None,
            started_at: None,
            ended_at: None,
            impact: None,
        }),
        tags: vec![],
        links: vec![],
        source: SourceRef {
            system: SourceSystem::Manual,
            url: None,
            opaque_id: None,
        },
    };

    let events = vec![pr, review, manual];
    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "mixed"]),
        title: "Mixed Work".into(),
        summary: Some("Cross-cutting work".into()),
        tags: vec![],
        stats: WorkstreamStats {
            pull_requests: 1,
            reviews: 1,
            manual_events: 1,
        },
        receipts: vec![
            events[0].id.clone(),
            events[1].id.clone(),
            events[2].id.clone(),
        ],
        events: vec![
            events[0].id.clone(),
            events[1].id.clone(),
            events[2].id.clone(),
        ],
    };
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = test_coverage("testuser", Completeness::Complete);

    let result = MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}

// ── Snapshot: CoverageFirst section ordering ────────────────────────────

#[test]
fn snapshot_coverage_first_ordering() {
    let events = vec![pr_event("owner/repo", 1, "Some PR")];
    let ws = WorkstreamFixture::new("Test Work")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = test_coverage("testuser", Completeness::Complete);

    let result = MarkdownRenderer::new()
        .with_section_order(shiplog_render_md::SectionOrder::CoverageFirst)
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    insta::assert_snapshot!(result);
}
