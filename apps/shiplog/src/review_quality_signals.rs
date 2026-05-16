use shiplog::ids::EventId;
use shiplog::schema::{
    event::{EventEnvelope, EventPayload},
    workstream::{Workstream, WorkstreamsFile},
};
use std::collections::HashMap;

#[derive(Default)]
pub(super) struct WorkstreamQualitySignals<'a> {
    pub(super) no_receipt_workstreams: Vec<&'a Workstream>,
    pub(super) broad_workstreams: Vec<&'a Workstream>,
    pub(super) manual_context_workstreams: Vec<&'a Workstream>,
    pub(super) thin_workstreams: Vec<&'a Workstream>,
    pub(super) large_misc_workstreams: Vec<&'a Workstream>,
    pub(super) code_only_workstreams: Vec<&'a Workstream>,
    pub(super) ticket_only_workstreams: Vec<&'a Workstream>,
    pub(super) manual_only_workstreams: Vec<&'a Workstream>,
    pub(super) too_many_receipt_workstreams: Vec<&'a Workstream>,
    pub(super) manual_events: usize,
}

#[derive(Default)]
struct WorkstreamEvidenceProfile {
    code: usize,
    tickets: usize,
    manual: usize,
}

const BROAD_WORKSTREAM_EVENT_THRESHOLD: usize = 10;
pub(super) const LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD: usize = 5;
pub(super) const TOO_MANY_SELECTED_RECEIPTS_THRESHOLD: usize = 5;
const SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD: usize = 5;

pub(super) fn workstream_quality_signals<'a>(
    workstreams: &'a WorkstreamsFile,
    events: &[EventEnvelope],
) -> WorkstreamQualitySignals<'a> {
    let events_by_id: HashMap<EventId, &EventEnvelope> = events
        .iter()
        .map(|event| (event.id.clone(), event))
        .collect();
    let manual_events = events
        .iter()
        .filter(|event| matches!(event.payload, EventPayload::Manual(_)))
        .count();
    let mut signals = WorkstreamQualitySignals {
        manual_events,
        ..WorkstreamQualitySignals::default()
    };

    for workstream in &workstreams.workstreams {
        let event_count = workstream.events.len();
        let profile = workstream_evidence_profile(workstream, &events_by_id);

        if workstream.receipts.is_empty() {
            signals.no_receipt_workstreams.push(workstream);
        }
        if event_count >= BROAD_WORKSTREAM_EVENT_THRESHOLD {
            signals.broad_workstreams.push(workstream);
        }
        if event_count == 1 && workstream.receipts.is_empty() {
            signals.thin_workstreams.push(workstream);
        }
        if event_count >= LARGE_MISC_WORKSTREAM_EVENT_THRESHOLD
            && is_misc_workstream_title(workstream)
        {
            signals.large_misc_workstreams.push(workstream);
        }
        if workstream.receipts.len() > TOO_MANY_SELECTED_RECEIPTS_THRESHOLD {
            signals.too_many_receipt_workstreams.push(workstream);
        }
        if event_count >= SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD
            && profile.code > 0
            && profile.tickets == 0
            && profile.manual == 0
        {
            signals.code_only_workstreams.push(workstream);
        }
        if event_count >= SINGLE_SOURCE_WORKSTREAM_EVENT_THRESHOLD
            && profile.tickets > 0
            && profile.code == 0
            && profile.manual == 0
        {
            signals.ticket_only_workstreams.push(workstream);
        }
        if event_count >= 1 && profile.manual > 0 && profile.code == 0 && profile.tickets == 0 {
            signals.manual_only_workstreams.push(workstream);
        }
        if event_count >= BROAD_WORKSTREAM_EVENT_THRESHOLD && profile.manual == 0 {
            signals.manual_context_workstreams.push(workstream);
        }
    }

    signals
}

fn workstream_evidence_profile(
    workstream: &Workstream,
    events_by_id: &HashMap<EventId, &EventEnvelope>,
) -> WorkstreamEvidenceProfile {
    let mut profile = WorkstreamEvidenceProfile::default();

    for event in workstream
        .events
        .iter()
        .filter_map(|event_id| events_by_id.get(event_id).copied())
    {
        match event_source_bucket(event) {
            WorkstreamSourceBucket::Code => profile.code += 1,
            WorkstreamSourceBucket::Ticket => profile.tickets += 1,
            WorkstreamSourceBucket::Manual => profile.manual += 1,
        }
    }

    profile
}

enum WorkstreamSourceBucket {
    Code,
    Ticket,
    Manual,
}

fn event_source_bucket(event: &EventEnvelope) -> WorkstreamSourceBucket {
    let source = event.source.system.as_str().to_ascii_lowercase();
    match source.as_str() {
        "jira" | "linear" => WorkstreamSourceBucket::Ticket,
        "manual" => WorkstreamSourceBucket::Manual,
        "github" | "gitlab" | "local_git" | "localgit" => WorkstreamSourceBucket::Code,
        _ => match event.payload {
            EventPayload::PullRequest(_) | EventPayload::Review(_) => WorkstreamSourceBucket::Code,
            EventPayload::Manual(_) => WorkstreamSourceBucket::Manual,
        },
    }
}

fn is_misc_workstream_title(workstream: &Workstream) -> bool {
    let normalized = workstream.title.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "misc" | "miscellaneous" | "other" | "uncategorized" | "untriaged"
    )
}
