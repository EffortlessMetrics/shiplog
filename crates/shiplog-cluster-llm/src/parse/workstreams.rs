use super::claims::ClaimTracker;
use super::stats;
use super::{LlmWorkstream, MAX_RECEIPTS_PER_WORKSTREAM};
use shiplog_ids::WorkstreamId;
use shiplog_schema::event::EventEnvelope;
use shiplog_schema::workstream::Workstream;

pub(super) fn build_claimed_workstreams(
    llm_workstreams: Vec<LlmWorkstream>,
    events: &[EventEnvelope],
    claims: &mut ClaimTracker,
) -> Vec<Workstream> {
    llm_workstreams
        .into_iter()
        .enumerate()
        .filter_map(|(index, llm_workstream)| {
            build_claimed_workstream(index, llm_workstream, events, claims)
        })
        .collect()
}

pub(super) fn build_uncategorized_workstream(
    events: &[EventEnvelope],
    orphan_indices: &[usize],
) -> Option<Workstream> {
    if orphan_indices.is_empty() {
        return None;
    }

    let selection = stats::summarize_events(events, orphan_indices);
    let receipts = stats::receipt_ids_for_indices(
        events,
        orphan_indices.iter().copied(),
        MAX_RECEIPTS_PER_WORKSTREAM,
    );

    Some(Workstream {
        id: WorkstreamId::from_parts(["llm", "uncategorized"]),
        title: "Uncategorized".to_string(),
        summary: Some("Events not assigned to any thematic workstream".to_string()),
        tags: vec!["uncategorized".to_string()],
        stats: selection.stats,
        events: selection.event_ids,
        receipts,
    })
}

fn build_claimed_workstream(
    workstream_index: usize,
    llm_workstream: LlmWorkstream,
    events: &[EventEnvelope],
    claims: &mut ClaimTracker,
) -> Option<Workstream> {
    let valid_event_indices = claims.claim_available_indices(llm_workstream.event_indices);
    if valid_event_indices.is_empty() {
        return None;
    }

    let valid_receipt_indices = claimed_receipt_indices(
        llm_workstream.receipt_indices,
        &valid_event_indices,
        MAX_RECEIPTS_PER_WORKSTREAM,
    );
    let selection = stats::summarize_events(events, &valid_event_indices);
    let receipt_ids = stats::receipt_ids_for_indices(events, valid_receipt_indices, usize::MAX);

    Some(Workstream {
        id: WorkstreamId::from_parts(["llm", &workstream_index.to_string()]),
        title: llm_workstream.title,
        summary: llm_workstream.summary,
        tags: llm_workstream.tags,
        stats: selection.stats,
        events: selection.event_ids,
        receipts: receipt_ids,
    })
}

fn claimed_receipt_indices(
    receipt_indices: Vec<usize>,
    valid_event_indices: &[usize],
    limit: usize,
) -> Vec<usize> {
    receipt_indices
        .into_iter()
        .filter(|index| valid_event_indices.contains(index))
        .take(limit)
        .collect()
}
