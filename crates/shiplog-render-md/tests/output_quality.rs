//! Output-quality tests for the Markdown renderer.
//!
//! Validates heading hierarchy, link formatting, workstream stats rendering,
//! special character handling, long title behaviour, and multi-profile output.

use chrono::{TimeZone, Utc};
use shiplog_ids::{EventId, WorkstreamId};
use shiplog_ports::Renderer;
use shiplog_render_md::{MarkdownRenderer, SectionOrder};
use shiplog_schema::{
    coverage::{Completeness, CoverageManifest, TimeWindow},
    event::*,
    workstream::{Workstream, WorkstreamStats, WorkstreamsFile},
};
use shiplog_testkit::{
    fixtures::WorkstreamFixture,
    pr_event,
};

// ── Helpers ──────────────────────────────────────────────────────────────

fn deterministic_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: shiplog_ids::RunId("quality_test_run".into()),
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        user: "testuser".into(),
        window: TimeWindow {
            since: chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness: Completeness::Complete,
    }
}

fn empty_workstreams() -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![],
    }
}

fn render(
    events: &[EventEnvelope],
    workstreams: &WorkstreamsFile,
    coverage: &CoverageManifest,
) -> String {
    MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", events, workstreams, coverage)
        .expect("render should succeed")
}

// ── Heading hierarchy (#, ##, ###) ──────────────────────────────────────

#[test]
fn heading_hierarchy_h1_summary_is_top_level() {
    let md = render(&[], &empty_workstreams(), &deterministic_coverage());
    assert!(
        md.contains("\n# Summary\n") || md.starts_with("# Summary\n"),
        "Summary should be an H1 heading"
    );
}

#[test]
fn heading_hierarchy_sections_are_h2() {
    let md = render(&[], &empty_workstreams(), &deterministic_coverage());
    for section in &[
        "## Workstreams",
        "## Receipts",
        "## Coverage",
        "## Appendix: All Receipts",
        "## File Artifacts",
    ] {
        assert!(md.contains(section), "Missing H2 heading: {section}");
    }
}

#[test]
fn heading_hierarchy_workstream_titles_are_h3() {
    let events = vec![pr_event("acme/repo", 1, "First")];
    let ws = WorkstreamFixture::new("My Feature")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    assert!(
        md.contains("### My Feature"),
        "Workstream titles should be H3"
    );
}

#[test]
fn heading_no_h4_or_deeper() {
    let events: Vec<_> = (1..=3)
        .map(|i| pr_event("acme/repo", i, &format!("PR {i}")))
        .collect();
    let mut builder = WorkstreamFixture::new("Deep test");
    for ev in &events {
        builder = builder.with_event(ev).with_receipt(ev);
    }
    let ws = builder.build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    for line in md.lines() {
        assert!(
            !line.starts_with("#### "),
            "Output should not contain H4 or deeper headings, found: {line}"
        );
    }
}

// ── Link formatting ─────────────────────────────────────────────────────

#[test]
fn links_use_markdown_format() {
    let events = vec![pr_event("acme/repo", 42, "Add caching")];
    let ws = WorkstreamFixture::new("Caching")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    // Receipt links should use [label](url) syntax
    assert!(
        md.contains("[acme/repo](https://github.com/acme/repo/pull/42)"),
        "Receipt links should use Markdown link syntax"
    );
}

#[test]
fn links_absent_when_no_url() {
    // Event with no links at all
    let event = EventEnvelope {
        id: EventId::from_parts(["manual", "nolink"]),
        kind: EventKind::Manual,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "testuser".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Manual(ManualEvent {
            event_type: ManualEventType::Note,
            title: "Offsite planning".into(),
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
    let ws = WorkstreamFixture::new("Planning")
        .with_event(&event)
        .with_receipt(&event)
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&[event], &workstreams, &deterministic_coverage());

    // Should not contain broken Markdown links like [](...)
    assert!(
        !md.contains("[]("),
        "Should not render empty-label links"
    );
}

// ── Workstream stats rendering ──────────────────────────────────────────

#[test]
fn workstream_stats_line_format() {
    let events: Vec<_> = (1..=3)
        .map(|i| pr_event("acme/repo", i, &format!("PR {i}")))
        .collect();
    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "stats-test"]),
        title: "Stats Test".into(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats {
            pull_requests: 3,
            reviews: 2,
            manual_events: 1,
        },
        events: events.iter().map(|e| e.id.clone()).collect(),
        receipts: events.iter().map(|e| e.id.clone()).collect(),
    };
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    assert!(
        md.contains("_PRs: 3, Reviews: 2, Manual: 1_"),
        "Stats should be italic with PRs/Reviews/Manual format"
    );
}

#[test]
fn workstream_stats_zero_values() {
    let events = vec![pr_event("acme/repo", 1, "Solo PR")];
    let ws = Workstream {
        id: WorkstreamId::from_parts(["ws", "zero"]),
        title: "Zero Stats".into(),
        summary: None,
        tags: vec![],
        stats: WorkstreamStats {
            pull_requests: 0,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![events[0].id.clone()],
        receipts: vec![events[0].id.clone()],
    };
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    assert!(
        md.contains("_PRs: 0, Reviews: 0, Manual: 0_"),
        "Zero stats should still render"
    );
}

// ── Special characters in event titles (escaping) ───────────────────────

#[test]
fn snapshot_html_angle_brackets_in_title() {
    let events = vec![pr_event(
        "acme/repo",
        1,
        "Fix <div> rendering & HTML <script> injection",
    )];
    let ws = WorkstreamFixture::new("HTML in titles")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };

    let md = render(&events, &workstreams, &deterministic_coverage());
    insta::assert_snapshot!(md);
}

#[test]
fn snapshot_markdown_special_chars_in_title() {
    let events = vec![
        pr_event("acme/repo", 1, "Use `Option<T>` instead of null"),
        pr_event("acme/repo", 2, "Fix #123: [urgent] pipe | handling"),
        pr_event("acme/repo", 3, "Support **bold** and *italic* in docs"),
    ];
    let mut builder = WorkstreamFixture::new("Markdown syntax in titles");
    for ev in &events {
        builder = builder.with_event(ev).with_receipt(ev);
    }
    let ws = builder.build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };

    let md = render(&events, &workstreams, &deterministic_coverage());
    insta::assert_snapshot!(md);
}

// ── Very long titles ────────────────────────────────────────────────────

#[test]
fn snapshot_very_long_title() {
    let long_title = "A".repeat(300);
    let events = vec![pr_event("acme/repo", 1, &long_title)];
    let ws = WorkstreamFixture::new("Long Title Test")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };

    let md = render(&events, &workstreams, &deterministic_coverage());

    // The renderer should include the title (whether truncated or not)
    // but must not panic or produce empty output
    assert!(!md.is_empty(), "Output should not be empty for long titles");
    insta::assert_snapshot!(md);
}

#[test]
fn long_workstream_title_renders_successfully() {
    let long_ws_title = format!("Workstream: {}", "X".repeat(200));
    let events = vec![pr_event("acme/repo", 1, "PR")];
    let ws = WorkstreamFixture::new(&long_ws_title)
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };

    let md = render(&events, &workstreams, &deterministic_coverage());
    assert!(
        md.contains(&format!("### {long_ws_title}")),
        "Long workstream title should appear as H3"
    );
}

// ── Multiple profiles render different content ──────────────────────────

#[test]
fn different_section_orders_produce_different_output() {
    let events = vec![pr_event("acme/repo", 1, "Feature A")];
    let ws = WorkstreamFixture::new("Feature")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let coverage = deterministic_coverage();

    let default_md = MarkdownRenderer::new()
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    let coverage_first_md = MarkdownRenderer::new()
        .with_section_order(SectionOrder::CoverageFirst)
        .render_packet_markdown("testuser", "2025-Q1", &events, &workstreams, &coverage)
        .unwrap();

    // Same content blocks but different order
    assert_ne!(
        default_md, coverage_first_md,
        "Different section orders should produce different output"
    );

    // Both should contain all sections
    for section in &["# Summary", "## Workstreams", "## Coverage"] {
        assert!(default_md.contains(section));
        assert!(coverage_first_md.contains(section));
    }

    // Default: Summary before Coverage
    let default_sum = default_md.find("# Summary").unwrap();
    let default_cov = default_md.find("## Coverage").unwrap();
    assert!(default_sum < default_cov, "Default: Summary before Coverage");

    // CoverageFirst: Coverage before Summary
    let cf_cov = coverage_first_md.find("## Coverage").unwrap();
    let cf_sum = coverage_first_md.find("# Summary").unwrap();
    assert!(cf_cov < cf_sum, "CoverageFirst: Coverage before Summary");
}

#[test]
fn snapshot_multiple_workstreams_with_mixed_stats() {
    let pr1 = pr_event("acme/api", 1, "API auth endpoint");
    let pr2 = pr_event("acme/api", 2, "API rate limiting");
    let pr3 = pr_event("acme/web", 3, "Dashboard redesign");

    let review = EventEnvelope {
        id: EventId::from_parts(["review", "multi-1"]),
        kind: EventKind::Review,
        occurred_at: Utc.timestamp_opt(0, 0).unwrap(),
        actor: Actor {
            login: "reviewer".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "acme/api".into(),
            html_url: None,
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::Review(ReviewEvent {
            pull_number: 1,
            pull_title: "API auth endpoint".into(),
            submitted_at: Utc.timestamp_opt(0, 0).unwrap(),
            state: "approved".into(),
            window: None,
        }),
        tags: vec![],
        links: vec![Link {
            label: "pr".into(),
            url: "https://github.com/acme/api/pull/1".into(),
        }],
        source: SourceRef {
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    };

    let events = vec![pr1, pr2, pr3, review];

    let ws_api = Workstream {
        id: WorkstreamId::from_parts(["ws", "api"]),
        title: "API Platform".into(),
        summary: Some("Backend API improvements".into()),
        tags: vec!["backend".into()],
        stats: WorkstreamStats {
            pull_requests: 2,
            reviews: 1,
            manual_events: 0,
        },
        events: vec![
            events[0].id.clone(),
            events[1].id.clone(),
            events[3].id.clone(),
        ],
        receipts: vec![
            events[0].id.clone(),
            events[1].id.clone(),
            events[3].id.clone(),
        ],
    };

    let ws_web = Workstream {
        id: WorkstreamId::from_parts(["ws", "web"]),
        title: "Web Frontend".into(),
        summary: Some("Dashboard improvements".into()),
        tags: vec!["frontend".into()],
        stats: WorkstreamStats {
            pull_requests: 1,
            reviews: 0,
            manual_events: 0,
        },
        events: vec![events[2].id.clone()],
        receipts: vec![events[2].id.clone()],
    };

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws_api, ws_web],
    };

    let md = render(&events, &workstreams, &deterministic_coverage());
    insta::assert_snapshot!(md);
}

// ── Claim scaffold structure ────────────────────────────────────────────

#[test]
fn claim_scaffolds_present_for_each_workstream() {
    let events = vec![
        pr_event("acme/repo", 1, "PR A"),
        pr_event("acme/repo", 2, "PR B"),
    ];
    let ws1 = WorkstreamFixture::new("Alpha")
        .with_event(&events[0])
        .with_receipt(&events[0])
        .build();
    let ws2 = WorkstreamFixture::new("Beta")
        .with_event(&events[1])
        .with_receipt(&events[1])
        .build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws1, ws2],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    let scaffolds: Vec<_> = md.matches("**Claim scaffolds**").collect();
    assert_eq!(
        scaffolds.len(),
        2,
        "Each workstream should have a claim scaffold block"
    );

    for field in &["Problem: _fill_", "What I shipped: _fill_", "Why it mattered: _fill_", "Result: _fill_"] {
        assert!(md.contains(field), "Missing claim field: {field}");
    }
}

// ── Appendix link in receipts ───────────────────────────────────────────

#[test]
fn appendix_anchor_link_present_when_truncated() {
    // 8 receipts > limit of 5
    let events: Vec<_> = (1..=8)
        .map(|i| pr_event("acme/repo", i, &format!("PR {i}")))
        .collect();
    let mut builder = WorkstreamFixture::new("Big stream");
    for ev in &events {
        builder = builder.with_event(ev).with_receipt(ev);
    }
    let ws = builder.build();
    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc.timestamp_opt(0, 0).unwrap(),
        workstreams: vec![ws],
    };
    let md = render(&events, &workstreams, &deterministic_coverage());

    assert!(
        md.contains("[Appendix](#appendix-receipts)"),
        "Truncated receipts should link to appendix"
    );
    assert!(
        md.contains("## Appendix: All Receipts"),
        "Appendix heading should exist"
    );
}
