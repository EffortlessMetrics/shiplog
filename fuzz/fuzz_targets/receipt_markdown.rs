//! Fuzz harness for markdown receipt formatting.

#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_receipt::format_receipt_markdown;
use shiplog_schema::event::{
    Actor, EventEnvelope, EventKind, EventPayload, Link, ManualEvent, ManualEventType,
    PullRequestEvent, PullRequestState, RepoRef, RepoVisibility, ReviewEvent, SourceRef,
    SourceSystem,
};

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or(0) % 3;
    let split_a = data.len() / 3;
    let split_b = (data.len() * 2) / 3;

    let title_start = if data.is_empty() { 0 } else { 1 };
    let title_end = split_a.max(title_start).min(data.len());
    let repo_start = split_a.min(data.len());
    let repo_end = split_b.max(repo_start).min(data.len());
    let url_start = split_b.min(data.len());

    let title = String::from_utf8_lossy(&data[title_start..title_end]).to_string();
    let repo = String::from_utf8_lossy(&data[repo_start..repo_end]).to_string();
    let url = String::from_utf8_lossy(&data[url_start..]).to_string();

    let ts = chrono::DateTime::<chrono::Utc>::from_timestamp(1_735_689_600, 0)
        .expect("valid fuzz timestamp");
    let event = match selector {
        0 => EventEnvelope {
            id: shiplog_ids::EventId::from_parts(["fuzz", "pr", "1"]),
            kind: EventKind::PullRequest,
            occurred_at: ts,
            actor: Actor {
                login: "fuzzer".to_string(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo.clone(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: 1,
                title: title.clone(),
                state: PullRequestState::Merged,
                created_at: ts,
                merged_at: Some(ts),
                additions: Some(1),
                deletions: Some(1),
                changed_files: Some(1),
                touched_paths_hint: vec![],
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".to_string(),
                url: url.clone(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        },
        1 => EventEnvelope {
            id: shiplog_ids::EventId::from_parts(["fuzz", "review", "1"]),
            kind: EventKind::Review,
            occurred_at: ts,
            actor: Actor {
                login: "fuzzer".to_string(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo.clone(),
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: 1,
                pull_title: title.clone(),
                submitted_at: ts,
                state: "approved".to_string(),
                window: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "pr".to_string(),
                url: url.clone(),
            }],
            source: SourceRef {
                system: SourceSystem::Github,
                url: None,
                opaque_id: None,
            },
        },
        _ => EventEnvelope {
            id: shiplog_ids::EventId::from_parts(["fuzz", "manual", "1"]),
            kind: EventKind::Manual,
            occurred_at: ts,
            actor: Actor {
                login: "fuzzer".to_string(),
                id: None,
            },
            repo: RepoRef {
                full_name: repo,
                html_url: None,
                visibility: RepoVisibility::Private,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Other,
                title,
                description: None,
                started_at: None,
                ended_at: None,
                impact: None,
            }),
            tags: vec![],
            links: vec![Link {
                label: "ref".to_string(),
                url,
            }],
            source: SourceRef {
                system: SourceSystem::Manual,
                url: None,
                opaque_id: None,
            },
        },
    };

    let line = format_receipt_markdown(&event);
    assert!(line.starts_with("- "));
    assert!(!line.contains('\n'));
    assert!(!line.contains('\r'));
});
