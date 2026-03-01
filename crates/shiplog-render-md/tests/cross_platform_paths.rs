//! Cross-platform path handling tests for shiplog-render-md.
//!
//! The Markdown renderer is intentionally transport-agnostic — it produces
//! strings, not files. These tests verify that the renderer does not embed
//! platform-specific path separators in its output, and that rendered content
//! can be written to paths with special characters.

use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_ports::Renderer;
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog_schema::event::*;
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fn pr_event(id: &str, repo: &str, title: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["render-md-path", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "tester".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: repo.into(),
            html_url: Some(format!("https://github.com/{repo}")),
            visibility: RepoVisibility::Public,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 42,
            title: title.into(),
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
            system: SourceSystem::Github,
            url: None,
            opaque_id: None,
        },
    }
}

fn make_workstreams(events: &[EventEnvelope]) -> WorkstreamsFile {
    WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![Workstream {
            id: WorkstreamId::from_parts(["test", "ws"]),
            title: "Test Workstream".into(),
            summary: Some("A workstream for testing".into()),
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: events.len(),
                reviews: 0,
                manual_events: 0,
            },
            events: events.iter().map(|e| e.id.clone()).collect(),
            receipts: vec![],
        }],
    }
}

fn make_coverage() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("render-md-path-test"),
        generated_at: Utc::now(),
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
// Renderer output does not contain platform-specific separators
// ---------------------------------------------------------------------------

#[test]
fn rendered_markdown_contains_no_backslash_path_separators() {
    let events = vec![pr_event("1", "acme/frontend", "Add feature")];
    let ws = make_workstreams(&events);
    let cov = make_coverage();

    let renderer = MarkdownRenderer::new();
    let md = renderer
        .render_packet_markdown("tester", "2025-01-01..2025-02-01", &events, &ws, &cov)
        .unwrap();

    // URLs should use forward slashes, and no filesystem backslashes should appear
    for line in md.lines() {
        // Skip lines that might legitimately contain backslashes (e.g., escaped chars)
        if line.contains("https://") || line.contains("http://") {
            continue;
        }
        // Markdown output should not contain bare Windows-style backslash paths
        assert!(
            !line.contains("\\\\"),
            "rendered markdown should not contain backslash path separators: {line}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rendered output can be written to paths with special characters
// ---------------------------------------------------------------------------

#[test]
fn rendered_output_writable_to_path_with_spaces() {
    let events = vec![pr_event("1", "acme/repo", "Test PR")];
    let ws = make_workstreams(&events);
    let cov = make_coverage();

    let renderer = MarkdownRenderer::new();
    let md = renderer
        .render_packet_markdown("tester", "2025-01-01..2025-02-01", &events, &ws, &cov)
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("output with spaces").join("packet.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &md).unwrap();

    let read_back = std::fs::read_to_string(&path).unwrap();
    assert_eq!(md, read_back);
}

#[test]
fn rendered_output_writable_to_path_with_unicode() {
    let events = vec![pr_event("1", "acme/repo", "Test PR")];
    let ws = make_workstreams(&events);
    let cov = make_coverage();

    let renderer = MarkdownRenderer::new();
    let md = renderer
        .render_packet_markdown("tester", "2025-01-01..2025-02-01", &events, &ws, &cov)
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("出力パス").join("packet.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, &md).unwrap();

    let read_back = std::fs::read_to_string(&path).unwrap();
    assert_eq!(md, read_back);
}

// ---------------------------------------------------------------------------
// URLs in rendered output use forward slashes (not affected by OS)
// ---------------------------------------------------------------------------

#[test]
fn rendered_urls_use_forward_slashes() {
    let events = vec![pr_event("1", "acme/repo", "Add feature")];
    let ws = make_workstreams(&events);
    let cov = make_coverage();

    let renderer = MarkdownRenderer::new();
    let md = renderer
        .render_packet_markdown("tester", "2025-01-01..2025-02-01", &events, &ws, &cov)
        .unwrap();

    // Any GitHub URL in the output should use forward slashes
    for line in md.lines() {
        if line.contains("github.com") {
            assert!(
                !line.contains("github.com\\"),
                "GitHub URLs should use forward slashes: {line}"
            );
        }
    }
}
