use crate::event::EventKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::{EventId, WorkstreamId};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamStats {
    pub pull_requests: usize,
    pub reviews: usize,
    pub manual_events: usize,
}

impl WorkstreamStats {
    pub fn zero() -> Self {
        Self {
            pull_requests: 0,
            reviews: 0,
            manual_events: 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workstream {
    pub id: WorkstreamId,
    pub title: String,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub stats: WorkstreamStats,
    /// Event IDs that belong to this workstream.
    pub events: Vec<EventId>,
    /// Curated receipts (subset of events) used in the packet.
    pub receipts: Vec<EventId>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamsFile {
    pub version: u32,
    pub generated_at: DateTime<Utc>,
    pub workstreams: Vec<Workstream>,
}

impl Workstream {
    pub fn bump_stats(&mut self, kind: &EventKind) {
        match kind {
            EventKind::PullRequest => self.stats.pull_requests += 1,
            EventKind::Review => self.stats.reviews += 1,
            EventKind::Manual => self.stats.manual_events += 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shiplog_ids::WorkstreamId;

    fn empty_workstream() -> Workstream {
        Workstream {
            id: WorkstreamId::from_parts(["ws", "test"]),
            title: "test".into(),
            summary: None,
            tags: vec![],
            stats: WorkstreamStats::zero(),
            events: vec![],
            receipts: vec![],
        }
    }

    #[test]
    fn bump_stats_pull_request() {
        let mut ws = empty_workstream();
        ws.bump_stats(&EventKind::PullRequest);
        assert_eq!(ws.stats.pull_requests, 1);
        assert_eq!(ws.stats.reviews, 0);
        assert_eq!(ws.stats.manual_events, 0);
    }

    #[test]
    fn bump_stats_review() {
        let mut ws = empty_workstream();
        ws.bump_stats(&EventKind::Review);
        assert_eq!(ws.stats.pull_requests, 0);
        assert_eq!(ws.stats.reviews, 1);
        assert_eq!(ws.stats.manual_events, 0);
    }

    #[test]
    fn bump_stats_manual() {
        let mut ws = empty_workstream();
        ws.bump_stats(&EventKind::Manual);
        assert_eq!(ws.stats.pull_requests, 0);
        assert_eq!(ws.stats.reviews, 0);
        assert_eq!(ws.stats.manual_events, 1);
    }
}
