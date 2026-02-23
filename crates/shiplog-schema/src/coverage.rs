use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeWindow {
    /// Inclusive start date (YYYY-MM-DD).
    pub since: NaiveDate,
    /// Exclusive end date (YYYY-MM-DD).
    pub until: NaiveDate,
}

impl TimeWindow {
    pub fn contains(&self, d: NaiveDate) -> bool {
        d >= self.since && d < self.until
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Completeness {
    Complete,
    Partial,
    Unknown,
}

/// One query window and what happened.
///
/// This is intentionally verbose.
/// Coverage is a first-class output, not a footnote.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageSlice {
    pub window: TimeWindow,
    pub query: String,
    pub total_count: u64,
    pub fetched: u64,
    pub incomplete_results: Option<bool>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageManifest {
    pub run_id: shiplog_ids::RunId,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub user: String,
    pub window: TimeWindow,
    /// "created" or "merged".
    pub mode: String,
    pub sources: Vec<String>,
    pub slices: Vec<CoverageSlice>,
    pub warnings: Vec<String>,
    pub completeness: Completeness,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window() -> TimeWindow {
        TimeWindow {
            since: NaiveDate::from_ymd_opt(2025, 1, 10).unwrap(),
            until: NaiveDate::from_ymd_opt(2025, 1, 20).unwrap(),
        }
    }

    #[test]
    fn contains_inclusive_start_boundary() {
        let w = window();
        assert!(w.contains(w.since));
    }

    #[test]
    fn contains_exclusive_end_boundary() {
        let w = window();
        assert!(!w.contains(w.until));
    }

    #[test]
    fn contains_day_before_until() {
        let w = window();
        let day_before = w.until.pred_opt().unwrap();
        assert!(w.contains(day_before));
    }

    #[test]
    fn contains_before_window() {
        let w = window();
        let before = w.since.pred_opt().unwrap();
        assert!(!w.contains(before));
    }

    #[test]
    fn contains_after_window() {
        let w = window();
        let after = w.until.succ_opt().unwrap();
        assert!(!w.contains(after));
    }
}
