#![no_main]

use chrono::Utc;
use libfuzzer_sys::fuzz_target;
use shiplog_cluster_llm_prompt::{chunk_events, format_event_list, summarize_event, system_prompt};
use shiplog_ids::EventId;
use shiplog_schema::event::*;

fn make_event(num: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["cluster", "llm", "prompt", &num.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: Utc::now(),
        actor: Actor {
            login: "fuzz-user".into(),
            id: None,
        },
        repo: RepoRef {
            full_name: "org/repo".into(),
            html_url: None,
            visibility: RepoVisibility::Unknown,
        },
        payload: EventPayload::PullRequest(PullRequestEvent {
            number: num,
            title: format!("Event {num}"),
            state: PullRequestState::Merged,
            created_at: Utc::now(),
            merged_at: Some(Utc::now()),
            additions: Some(5),
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

fuzz_target!(|data: &[u8]| {
    let event_count = (data.first().copied().unwrap_or(0) as usize) % 64;
    let max_tokens = ((data.get(1).copied().unwrap_or(0) as usize) * 16) + 1;
    let max_workstreams = ((data.get(2).copied().unwrap_or(0) as usize) % 20) + 1;

    let events: Vec<EventEnvelope> = (0..event_count).map(make_event).collect();
    let rendered = format_event_list(&events);
    let chunks = chunk_events(&events, max_tokens);
    let flattened: Vec<usize> = chunks.iter().flatten().copied().collect();
    let expected: Vec<usize> = (0..events.len()).collect();

    if events.is_empty() {
        assert_eq!(rendered, "");
    } else {
        assert!(rendered.contains("[0]"));
        assert_eq!(flattened.len(), events.len());
    }

    assert_eq!(flattened, expected);
    if !events.is_empty() {
        let summary = summarize_event(&events[0]);
        assert!(summary.starts_with("PR#0"));
    }
    let limited_prompt = system_prompt(Some(max_workstreams));
    let unlimited_prompt = system_prompt(None);

    assert!(limited_prompt.contains(&format!("at most {max_workstreams}")));
    assert!(!unlimited_prompt.contains("at most"));
});
