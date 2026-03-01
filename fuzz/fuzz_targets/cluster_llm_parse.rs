#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_cluster_llm_parse::parse_llm_response;
use shiplog_ids::EventId;
use shiplog_schema::event::*;
use std::collections::HashSet;

fn make_event(num: u64) -> EventEnvelope {
    EventEnvelope {
        id: EventId::from_parts(["fuzz", "cluster-llm-parse", &num.to_string()]),
        kind: EventKind::PullRequest,
        occurred_at: chrono::Utc::now(),
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
            created_at: chrono::Utc::now(),
            merged_at: Some(chrono::Utc::now()),
            additions: Some(3),
            deletions: Some(1),
            changed_files: Some(2),
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
    let event_count = (data.first().copied().unwrap_or(0) as usize) % 24;
    let workstream_count = (data.get(1).copied().unwrap_or(0) as usize) % 6;

    let events: Vec<EventEnvelope> = (0..event_count).map(|idx| make_event(idx as u64)).collect();
    let mut cursor = 2;
    let mut workstreams = Vec::new();

    for _ in 0..workstream_count {
        let mut event_indices = Vec::new();
        let mut receipt_indices = Vec::new();

        let evt_count = (data.get(cursor).copied().unwrap_or(1) as usize) % 8 + 1;
        cursor += 1;
        for i in 0..evt_count {
            let idx = data.get(cursor + i).copied().unwrap_or(0) as usize;
            event_indices.push(idx % (event_count + 1));
            let receipt_idx = data.get(cursor + i + evt_count).copied().unwrap_or(0) as usize;
            receipt_indices.push(receipt_idx % (event_count + 1));
        }
        cursor += evt_count * 2;

        workstreams.push(serde_json::json!({
            "title": format!("ws-{}", cursor),
            "summary": "fuzz",
            "tags": [],
            "event_indices": event_indices,
            "receipt_indices": receipt_indices,
        }));
    }

    let payload = serde_json::json!({ "workstreams": workstreams }).to_string();
    let parsed = parse_llm_response(&payload, &events).unwrap();
    let assigned_ids: Vec<String> = parsed
        .workstreams
        .iter()
        .flat_map(|ws| ws.events.iter().map(|id| id.to_string()))
        .collect();

    let assigned_set: HashSet<String> = assigned_ids.iter().cloned().collect();
    assert_eq!(assigned_ids.len(), assigned_set.len(), "events must not duplicate across workstreams");
    assert_eq!(assigned_set.len(), event_count, "all events should be represented exactly once");
});
