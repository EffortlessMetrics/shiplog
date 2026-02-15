use anyhow::{Context, Result};
use chrono::Utc;
use shiplog_ids::WorkstreamId;
use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::{Workstream, WorkstreamStats, WorkstreamsFile};
use std::collections::BTreeSet;

#[derive(serde::Deserialize)]
pub(crate) struct LlmResponse {
    pub workstreams: Vec<LlmWorkstream>,
}

#[derive(serde::Deserialize)]
pub(crate) struct LlmWorkstream {
    pub title: String,
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub event_indices: Vec<usize>,
    #[serde(default)]
    pub receipt_indices: Vec<usize>,
}

/// Parse LLM JSON response into a WorkstreamsFile.
/// Invalid indices are skipped. Orphan events get an "Uncategorized" workstream.
pub fn parse_llm_response(json_str: &str, events: &[EventEnvelope]) -> Result<WorkstreamsFile> {
    let resp: LlmResponse =
        serde_json::from_str(json_str).context("parse LLM clustering response")?;

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

    // Collect orphans into an "Uncategorized" workstream
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
