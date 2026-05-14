use shiplog_schema::event::{EventEnvelope, EventKind};
use shiplog_schema::workstream::WorkstreamStats;

/// Projection of selected events into the fields carried by a workstream.
pub(super) struct EventSelection {
    pub(super) stats: WorkstreamStats,
    pub(super) event_ids: Vec<shiplog_ids::EventId>,
}

pub(super) fn summarize_events(events: &[EventEnvelope], indices: &[usize]) -> EventSelection {
    let mut stats = WorkstreamStats::zero();
    let mut event_ids = Vec::with_capacity(indices.len());

    for &index in indices {
        let event = &events[index];
        event_ids.push(event.id.clone());
        count_event_kind(&mut stats, &event.kind);
    }

    EventSelection { stats, event_ids }
}

pub(super) fn receipt_ids_for_indices(
    events: &[EventEnvelope],
    indices: impl IntoIterator<Item = usize>,
    limit: usize,
) -> Vec<shiplog_ids::EventId> {
    indices
        .into_iter()
        .take(limit)
        .map(|index| events[index].id.clone())
        .collect()
}

fn count_event_kind(stats: &mut WorkstreamStats, kind: &EventKind) {
    match kind {
        EventKind::PullRequest => stats.pull_requests += 1,
        EventKind::Review => stats.reviews += 1,
        EventKind::Manual => stats.manual_events += 1,
    }
}
