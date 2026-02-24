use chrono::{NaiveDate, Utc};
use shiplog_ids::{EventId, RunId, WorkstreamId};
use shiplog_render_md::MarkdownRenderer;
use shiplog_schema::{
    coverage::{CoverageManifest, TimeWindow},
    event::{
        Actor, EventEnvelope, EventKind, EventPayload, PullRequestEvent, PullRequestState, RepoRef,
        SourceRef, SourceSystem,
    },
    workstream::{Workstream, WorkstreamStats, WorkstreamsFile},
};
use shiplog_workstream_receipt_policy::WORKSTREAM_RECEIPT_RENDER_LIMIT;

fn pull_request_event(id: &str) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["render-md", id]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "agent".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "repo/policy".into(),
            html_url: Some("https://example.com/repo/policy".into()),
            visibility: shiplog_schema::event::RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: 101,
            title: "Integration PR".into(),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(1),
            deletions: Some(0),
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

fn coverage_manifest() -> CoverageManifest {
    CoverageManifest {
        run_id: RunId::now("render-md-receipts"),
        generated_at: Utc::now(),
        user: "agent".into(),
        window: TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
        },
        mode: "merged".into(),
        sources: vec!["github".into()],
        slices: vec![],
        warnings: vec![],
        completeness: shiplog_schema::coverage::Completeness::Complete,
    }
}

#[test]
fn markdown_renderer_shows_only_render_limit_in_receipt_summary() {
    let events = (0..10)
        .map(|i| pull_request_event(&i.to_string()))
        .collect::<Vec<_>>();
    let hidden_receipt_count = events.len() - WORKSTREAM_RECEIPT_RENDER_LIMIT;
    let event_ids = events.iter().map(|event| event.id.clone()).collect::<Vec<_>>();

    let ws = Workstream {
        id: WorkstreamId::from_parts(["repo", "policy"]),
        title: "repo/policy".into(),
        summary: None,
        tags: vec!["repo".into()],
        stats: WorkstreamStats {
            pull_requests: event_ids.len(),
            reviews: 0,
            manual_events: 0,
        },
        events: event_ids.clone(),
        receipts: event_ids,
    };

    let workstreams = WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams: vec![ws],
    };

    let markdown = MarkdownRenderer::new().render_packet_markdown(
        "agent",
        "2026-02-24",
        &events,
        &workstreams,
        &coverage_manifest(),
    )
    .unwrap();

    assert!(markdown.contains(&format!(
        "- *... and {} more in [Appendix](#appendix-receipts)*",
        hidden_receipt_count
    )));
}
