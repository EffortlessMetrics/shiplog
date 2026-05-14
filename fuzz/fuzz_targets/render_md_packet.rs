//! Fuzz harness for the full Markdown rendering pipeline.
//!
//! Exercises `MarkdownRenderer::render_packet_markdown` with fuzz-derived
//! events, workstreams, and coverage metadata. The renderer must never panic
//! regardless of input shape.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog::ports::Renderer;
use shiplog::render::md::MarkdownRenderer;
use shiplog::schema::coverage::{Completeness, CoverageManifest, TimeWindow};
use shiplog::schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, PullRequestEvent, PullRequestState,
    RepoRef, RepoVisibility, ReviewEvent, SourceRef, SourceSystem,
};
use shiplog::schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let ts = chrono::DateTime::<chrono::Utc>::from_timestamp(1_735_689_600, 0)
        .expect("valid fuzz timestamp");
    let since = chrono::NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
    let until = chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();

    // Use first byte to control event count and types
    let event_count = (data[0] % 5) as usize;
    let ws_count = (data[1] % 3) as usize;
    let completeness_sel = data[2] % 3;
    let order_sel = data[3] % 2;

    let rest = &data[4..];

    // Build fuzz-derived strings from remaining bytes
    let chunk_size = if rest.is_empty() {
        0
    } else {
        rest.len() / (event_count + ws_count + 1).max(1)
    };

    let mut events = Vec::new();
    for i in 0..event_count {
        let start = i * chunk_size;
        let end = ((i + 1) * chunk_size).min(rest.len());
        let title = String::from_utf8_lossy(rest.get(start..end).unwrap_or_default()).to_string();
        let kind_sel = rest.get(start).copied().unwrap_or(0) % 3;

        let event = match kind_sel {
            0 => EventEnvelope {
                id: shiplog::ids::EventId::from_parts(["fuzz", "pr", &i.to_string()]),
                kind: EventKind::PullRequest,
                occurred_at: ts,
                actor: Actor {
                    login: "fuzzer".into(),
                    id: None,
                },
                repo: RepoRef {
                    full_name: "fuzz/repo".into(),
                    html_url: None,
                    visibility: RepoVisibility::Private,
                },
                payload: EventPayload::PullRequest(PullRequestEvent {
                    number: i as u64 + 1,
                    title,
                    state: PullRequestState::Merged,
                    created_at: ts,
                    merged_at: Some(ts),
                    additions: Some(1),
                    deletions: Some(0),
                    changed_files: Some(1),
                    touched_paths_hint: vec![],
                    window: None,
                }),
                tags: vec![],
                links: vec![Link {
                    label: "pr".into(),
                    url: format!("https://github.com/fuzz/repo/pull/{}", i + 1),
                }],
                source: SourceRef {
                    system: SourceSystem::Github,
                    url: None,
                    opaque_id: None,
                },
            },
            1 => EventEnvelope {
                id: shiplog::ids::EventId::from_parts(["fuzz", "review", &i.to_string()]),
                kind: EventKind::Review,
                occurred_at: ts,
                actor: Actor {
                    login: "fuzzer".into(),
                    id: None,
                },
                repo: RepoRef {
                    full_name: "fuzz/repo".into(),
                    html_url: None,
                    visibility: RepoVisibility::Private,
                },
                payload: EventPayload::Review(ReviewEvent {
                    pull_number: 1,
                    pull_title: title,
                    submitted_at: ts,
                    state: "approved".into(),
                    window: None,
                }),
                tags: vec![],
                links: vec![],
                source: SourceRef {
                    system: SourceSystem::Github,
                    url: None,
                    opaque_id: None,
                },
            },
            _ => EventEnvelope {
                id: shiplog::ids::EventId::from_parts(["fuzz", "manual", &i.to_string()]),
                kind: EventKind::Manual,
                occurred_at: ts,
                actor: Actor {
                    login: "fuzzer".into(),
                    id: None,
                },
                repo: RepoRef {
                    full_name: "fuzz/repo".into(),
                    html_url: None,
                    visibility: RepoVisibility::Private,
                },
                payload: EventPayload::Manual(shiplog::schema::event::ManualEvent {
                    event_type: shiplog::schema::event::ManualEventType::Other,
                    title,
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
            },
        };
        events.push(event);
    }

    // Build workstreams referencing fuzzed event IDs
    let mut workstreams = Vec::new();
    for w in 0..ws_count {
        let start = (event_count + w) * chunk_size;
        let end = ((event_count + w + 1) * chunk_size).min(rest.len());
        let title = String::from_utf8_lossy(rest.get(start..end).unwrap_or_default()).to_string();

        let event_ids: Vec<shiplog::ids::EventId> =
            events.iter().take(w + 1).map(|e| e.id.clone()).collect();
        workstreams.push(Workstream {
            id: shiplog::ids::WorkstreamId::from_parts(["fuzz", "ws", &w.to_string()]),
            title,
            summary: Some("fuzz summary".into()),
            tags: vec![],
            stats: WorkstreamStats {
                pull_requests: events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::PullRequest))
                    .count(),
                reviews: events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::Review))
                    .count(),
                manual_events: events
                    .iter()
                    .filter(|e| matches!(e.kind, EventKind::Manual))
                    .count(),
            },
            events: event_ids.clone(),
            receipts: event_ids,
        });
    }

    let ws_file = WorkstreamsFile {
        version: 1,
        generated_at: ts,
        workstreams,
    };

    let completeness = match completeness_sel {
        0 => Completeness::Complete,
        1 => Completeness::Partial,
        _ => Completeness::Unknown,
    };

    let coverage = CoverageManifest {
        run_id: shiplog::ids::RunId("fuzz-run".into()),
        generated_at: ts,
        user: "fuzzer".into(),
        window: TimeWindow { since, until },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness,
    };

    let renderer = if order_sel == 0 {
        MarkdownRenderer::new()
    } else {
        MarkdownRenderer::new().with_section_order(shiplog::render::md::SectionOrder::CoverageFirst)
    };

    // Must not panic
    let result = renderer.render_packet_markdown(
        "fuzzer",
        "2025-01-01..2025-04-01",
        &events,
        &ws_file,
        &coverage,
    );
    if let Ok(md) = result {
        assert!(!md.is_empty(), "rendered markdown should not be empty");
        assert!(
            md.contains("# Summary"),
            "output should contain Summary heading"
        );
    }
});
