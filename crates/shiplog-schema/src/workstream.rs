use crate::event::EventKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::{EventId, WorkstreamId};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamStats {
    pub pull_requests: usize,
    pub reviews: usize,
}

impl WorkstreamStats {
    pub fn zero() -> Self {
        Self {
            pull_requests: 0,
            reviews: 0,
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
        }
    }
}
