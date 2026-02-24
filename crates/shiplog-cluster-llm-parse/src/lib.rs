use anyhow::{Context, Result};
use chrono::Utc;
use serde::Deserialize;
use shiplog_ids::WorkstreamId;
use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::collections::BTreeSet;

#[derive(Deserialize)]
struct LlmResponse {
    workstreams: Vec<LlmWorkstream>,
}

#[derive(Deserialize)]
struct LlmWorkstream {
    title: String,
    summary: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    event_indices: Vec<usize>,
    #[serde(default)]
    receipt_indices: Vec<usize>,
}

/// Parse the LLM response payload into a `WorkstreamsFile`.
///
/// - Invalid indices are ignored.
/// - Duplicate claims across workstreams follow first-wins semantics.
/// - Up to 10 receipt IDs are preserved per workstream.
/// - Any unclaimed input events are grouped under `Uncategorized`.
pub fn parse_llm_response(json_str: &str, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
    let resp: LlmResponse = serde_json::from_str(json_str).context("parse LLM clustering response")?;

    let mut claimed: BTreeSet<usize> = BTreeSet::new();
    let mut workstreams = Vec::new();

    for (ws_idx, llm_ws) in resp.workstreams.into_iter().enumerate() {
        let valid_indices: Vec<usize> = llm_ws
            .event_indices
            .into_iter()
            .filter(|&i| i < events.len() && !claimed.contains(&i))
            .collect();

        for &i in &valid_indices {
            claimed.insert(i);
        }

        if valid_indices.is_empty() {
            continue;
        }

        let valid_receipts: Vec<usize> = llm_ws
            .receipt_indices
            .into_iter()
            .filter(|i| valid_indices.contains(i))
            .take(10)
            .collect();

        let id = WorkstreamId::from_parts(["llm", &ws_idx.to_string()]);
        let mut stats = WorkstreamStats::zero();
        let mut event_ids = Vec::new();
        let mut receipt_ids = Vec::new();

        for &i in &valid_indices {
            let ev = &events[i];
            event_ids.push(ev.id.clone());
            match ev.kind {
                EventKind::PullRequest => stats.pull_requests += 1,
                EventKind::Review => stats.reviews += 1,
                EventKind::Manual => stats.manual_events += 1,
            }
        }

        for &i in &valid_receipts {
            receipt_ids.push(events[i].id.clone());
        }

        workstreams.push(Workstream {
            id,
            title: llm_ws.title,
            summary: llm_ws.summary,
            tags: llm_ws.tags,
            stats,
            events: event_ids,
            receipts: receipt_ids,
        });
    }

    let orphans: Vec<usize> = (0..events.len()).filter(|i| !claimed.contains(i)).collect();

    if !orphans.is_empty() {
        let id = WorkstreamId::from_parts(["llm", "uncategorized"]);
        let mut stats = WorkstreamStats::zero();
        let mut event_ids = Vec::new();
        let mut receipt_ids = Vec::new();

        for &i in &orphans {
            let ev = &events[i];
            event_ids.push(ev.id.clone());
            match ev.kind {
                EventKind::PullRequest => stats.pull_requests += 1,
                EventKind::Review => stats.reviews += 1,
                EventKind::Manual => stats.manual_events += 1,
            }
            if receipt_ids.len() < 10 {
                receipt_ids.push(ev.id.clone());
            }
        }

        workstreams.push(Workstream {
            id,
            title: "Uncategorized".to_string(),
            summary: Some("Events not assigned to any thematic workstream".to_string()),
            tags: vec!["uncategorized".to_string()],
            stats,
            events: event_ids,
            receipts: receipt_ids,
        });
    }

    Ok(WorkstreamsFile {
        version: 1,
        generated_at: Utc::now(),
        workstreams,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use shiplog_ids::EventId;
    use shiplog_schema::event::*;

    fn make_pr_event(num: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", "pr", &num.to_string()]),
            kind: EventKind::PullRequest,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "org/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::PullRequest(PullRequestEvent {
                number: num,
                title: format!("PR {num}"),
                state: PullRequestState::Merged,
                created_at: Utc::now(),
                merged_at: Some(Utc::now()),
                additions: Some(10),
                deletions: Some(5),
                changed_files: Some(3),
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

    fn make_review_event(num: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", "review", &num.to_string()]),
            kind: EventKind::Review,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "reviewer".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "org/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::Review(ReviewEvent {
                pull_number: num,
                pull_title: format!("PR {num}"),
                submitted_at: Utc::now(),
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
        }
    }

    fn make_manual_event(num: u64) -> EventEnvelope {
        EventEnvelope {
            id: EventId::from_parts(["test", "manual", &num.to_string()]),
            kind: EventKind::Manual,
            occurred_at: Utc::now(),
            actor: Actor {
                login: "user".into(),
                id: None,
            },
            repo: RepoRef {
                full_name: "org/repo".into(),
                html_url: None,
                visibility: RepoVisibility::Unknown,
            },
            payload: EventPayload::Manual(ManualEvent {
                event_type: ManualEventType::Note,
                title: format!("Manual {num}"),
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
        }
    }

    #[test]
    fn mixed_event_types_stats_counted_exactly() {
        let events = vec![make_pr_event(1), make_review_event(2), make_manual_event(3)];

        let json = serde_json::json!({
            "workstreams": [{
                "title": "Mixed",
                "summary": "All types",
                "tags": [],
                "event_indices": [0, 1, 2],
                "receipt_indices": [0]
            }]
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        let ws = &result.workstreams[0];
        assert_eq!(ws.stats.pull_requests, 1, "PR count must be exactly 1");
        assert_eq!(ws.stats.reviews, 1, "review count must be exactly 1");
        assert_eq!(ws.stats.manual_events, 1, "manual count must be exactly 1");
    }

    #[test]
    fn index_at_events_len_is_skipped() {
        let events = vec![make_pr_event(1)];

        let json = serde_json::json!({
            "workstreams": [{
                "title": "Boundary",
                "summary": "test",
                "tags": [],
                "event_indices": [1],
                "receipt_indices": []
            }]
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        assert_eq!(result.workstreams[0].title, "Uncategorized");
        assert_eq!(result.workstreams[0].events.len(), 1);
    }

    #[test]
    fn orphan_receipts_capped_at_10() {
        let events: Vec<EventEnvelope> = (0..15).map(make_pr_event).collect();

        let json = serde_json::json!({
            "workstreams": []
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        let orphan_ws = &result.workstreams[0];
        assert_eq!(orphan_ws.title, "Uncategorized");
        assert_eq!(orphan_ws.events.len(), 15);
        assert_eq!(orphan_ws.receipts.len(), 10, "orphan receipts capped at 10");
    }

    #[test]
    fn orphan_stats_count_mixed_types() {
        let events = vec![make_pr_event(1), make_review_event(2), make_manual_event(3)];

        let json = serde_json::json!({
            "workstreams": []
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        let ws = &result.workstreams[0];
        assert_eq!(ws.stats.pull_requests, 1);
        assert_eq!(ws.stats.reviews, 1);
        assert_eq!(ws.stats.manual_events, 1);
    }

    #[test]
    fn duplicate_index_claimed_only_once() {
        let events = vec![make_pr_event(1), make_pr_event(2)];

        let json = serde_json::json!({
            "workstreams": [
                {
                    "title": "First",
                    "event_indices": [0, 1],
                    "receipt_indices": []
                },
                {
                    "title": "Second",
                    "event_indices": [0, 1],
                    "receipt_indices": []
                }
            ]
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        assert_eq!(result.workstreams[0].title, "First");
        assert_eq!(result.workstreams[0].events.len(), 2);
    }

    #[test]
    fn empty_events_no_workstreams() {
        let json = serde_json::json!({
            "workstreams": [{
                "title": "Empty",
                "event_indices": [0],
                "receipt_indices": []
            }]
        });

        let result = parse_llm_response(&json.to_string(), &[]).unwrap();
        assert_eq!(result.workstreams.len(), 0);
    }

    #[test]
    fn receipt_indices_filtered_to_valid_events() {
        let events = vec![make_pr_event(1), make_pr_event(2)];

        let json = serde_json::json!({
            "workstreams": [{
                "title": "Receipts",
                "event_indices": [0],
                "receipt_indices": [0, 1]
            }]
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        let ws = &result.workstreams[0];
        assert_eq!(ws.receipts.len(), 1);
        assert_eq!(result.workstreams.len(), 2);
        assert_eq!(result.workstreams[1].title, "Uncategorized");
    }

    #[test]
    fn receipt_indices_capped_at_10_for_claimed_workstream() {
        let events: Vec<EventEnvelope> = (0..15).map(make_pr_event).collect();
        let indices: Vec<usize> = (0..15).collect();

        let json = serde_json::json!({
            "workstreams": [{
                "title": "Many receipts",
                "event_indices": indices,
                "receipt_indices": indices
            }]
        });

        let result = parse_llm_response(&json.to_string(), &events).unwrap();
        assert_eq!(result.workstreams.len(), 1);
        assert_eq!(result.workstreams[0].receipts.len(), 10);
    }
}
