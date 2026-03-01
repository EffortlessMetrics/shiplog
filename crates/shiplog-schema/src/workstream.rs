use crate::event::EventKind;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use shiplog_ids::{EventId, WorkstreamId};

/// Aggregate counters for events within a workstream.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamStats {
    /// Number of pull requests in this workstream.
    pub pull_requests: usize,
    /// Number of code reviews in this workstream.
    pub reviews: usize,
    /// Number of manually-entered events in this workstream.
    pub manual_events: usize,
}

impl WorkstreamStats {
    /// Returns a stats struct with all counters set to zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::workstream::WorkstreamStats;
    ///
    /// let s = WorkstreamStats::zero();
    /// assert_eq!(s.pull_requests, 0);
    /// assert_eq!(s.reviews, 0);
    /// assert_eq!(s.manual_events, 0);
    /// ```
    pub fn zero() -> Self {
        Self {
            pull_requests: 0,
            reviews: 0,
            manual_events: 0,
        }
    }
}

/// A logical grouping of related events (e.g. all work on one repository).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workstream {
    /// Deterministic identifier for this workstream.
    pub id: WorkstreamId,
    /// Human-readable title.
    pub title: String,
    /// Optional narrative summary.
    pub summary: Option<String>,
    /// Freeform tags for categorisation.
    pub tags: Vec<String>,
    /// Aggregate event counters.
    pub stats: WorkstreamStats,
    /// Event IDs that belong to this workstream.
    pub events: Vec<EventId>,
    /// Curated receipts (subset of events) used in the packet.
    pub receipts: Vec<EventId>,
}

/// Top-level workstreams file used for persistence and curation.
///
/// # Examples
///
/// ```
/// use shiplog_schema::workstream::{WorkstreamsFile, Workstream, WorkstreamStats};
/// use shiplog_ids::WorkstreamId;
/// use chrono::Utc;
///
/// let file = WorkstreamsFile {
///     version: 1,
///     generated_at: Utc::now(),
///     workstreams: vec![Workstream {
///         id: WorkstreamId::from_parts(["repo", "acme/widgets"]),
///         title: "widgets".into(),
///         summary: Some("Widget work".into()),
///         tags: vec!["infra".into()],
///         stats: WorkstreamStats::zero(),
///         events: vec![],
///         receipts: vec![],
///     }],
/// };
/// assert_eq!(file.workstreams.len(), 1);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkstreamsFile {
    /// Schema version for forward compatibility.
    pub version: u32,
    /// Timestamp when this file was generated.
    pub generated_at: DateTime<Utc>,
    /// Ordered list of workstreams.
    pub workstreams: Vec<Workstream>,
}

impl Workstream {
    /// Increment the stat counter for the given event kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::workstream::{Workstream, WorkstreamStats};
    /// use shiplog_schema::event::EventKind;
    /// use shiplog_ids::WorkstreamId;
    ///
    /// let mut ws = Workstream {
    ///     id: WorkstreamId::from_parts(["repo", "acme/widgets"]),
    ///     title: "widgets".into(),
    ///     summary: None,
    ///     tags: vec![],
    ///     stats: WorkstreamStats::zero(),
    ///     events: vec![],
    ///     receipts: vec![],
    /// };
    /// ws.bump_stats(&EventKind::PullRequest);
    /// ws.bump_stats(&EventKind::PullRequest);
    /// ws.bump_stats(&EventKind::Review);
    /// assert_eq!(ws.stats.pull_requests, 2);
    /// assert_eq!(ws.stats.reviews, 1);
    /// ```
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

    #[test]
    fn workstream_stats_zero_returns_all_zeros() {
        let stats = WorkstreamStats::zero();
        assert_eq!(stats.pull_requests, 0);
        assert_eq!(stats.reviews, 0);
        assert_eq!(stats.manual_events, 0);
    }

    #[test]
    fn workstream_stats_serde_roundtrip() {
        let stats = WorkstreamStats {
            pull_requests: 5,
            reviews: 3,
            manual_events: 2,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let back: WorkstreamStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats, back);
    }

    #[test]
    fn workstream_serde_roundtrip() {
        let ws = Workstream {
            id: WorkstreamId::from_parts(["ws", "auth"]),
            title: "Auth work".into(),
            summary: Some("OAuth2".into()),
            tags: vec!["security".into()],
            stats: WorkstreamStats {
                pull_requests: 2,
                reviews: 1,
                manual_events: 0,
            },
            events: vec![shiplog_ids::EventId::from_parts(["e1"])],
            receipts: vec![],
        };
        let json = serde_json::to_string(&ws).unwrap();
        let back: Workstream = serde_json::from_str(&json).unwrap();
        assert_eq!(ws, back);
    }

    #[test]
    fn workstreams_file_serde_roundtrip() {
        let file = WorkstreamsFile {
            version: 1,
            generated_at: chrono::Utc::now(),
            workstreams: vec![],
        };
        let json = serde_json::to_string(&file).unwrap();
        let back: WorkstreamsFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file, back);
    }
}
