use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// A half-open date range `[since, until)` used for query windows.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeWindow {
    /// Inclusive start date (YYYY-MM-DD).
    pub since: NaiveDate,
    /// Exclusive end date (YYYY-MM-DD).
    pub until: NaiveDate,
}

impl TimeWindow {
    /// Returns `true` if `d` falls within the half-open range `[since, until)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use shiplog_schema::coverage::TimeWindow;
    /// use chrono::NaiveDate;
    ///
    /// let w = TimeWindow {
    ///     since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    ///     until: NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
    /// };
    ///
    /// assert!(w.contains(NaiveDate::from_ymd_opt(2025, 1, 15).unwrap()));
    /// assert!(w.contains(w.since));     // inclusive start
    /// assert!(!w.contains(w.until));    // exclusive end
    /// ```
    pub fn contains(&self, d: NaiveDate) -> bool {
        d >= self.since && d < self.until
    }
}

/// How complete the data fetch was for a given window.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Completeness {
    /// All expected data was fetched.
    Complete,
    /// Some data may be missing.
    Partial,
    /// Completeness could not be determined.
    Unknown,
}

/// One query window and what happened.
///
/// This is intentionally verbose.
/// Coverage is a first-class output, not a footnote.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageSlice {
    /// Time window this slice covers.
    pub window: TimeWindow,
    /// The search query that produced this slice.
    pub query: String,
    /// Total results reported by the source.
    pub total_count: u64,
    /// How many results were actually fetched.
    pub fetched: u64,
    /// Whether the source flagged results as incomplete.
    pub incomplete_results: Option<bool>,
    /// Free-text notes about this slice.
    pub notes: Vec<String>,
}

/// The coverage manifest for a run.
///
/// # Examples
///
/// ```
/// use shiplog_schema::coverage::*;
/// use shiplog_ids::RunId;
/// use chrono::{NaiveDate, Utc};
///
/// let manifest = CoverageManifest {
///     run_id: RunId::now("test"),
///     generated_at: Utc::now(),
///     user: "octocat".into(),
///     window: TimeWindow {
///         since: NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
///         until: NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
///     },
///     mode: "merged".into(),
///     sources: vec!["github".into()],
///     slices: vec![],
///     warnings: vec![],
///     completeness: Completeness::Complete,
/// };
/// assert_eq!(manifest.user, "octocat");
/// assert_eq!(manifest.completeness, Completeness::Complete);
/// ```
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoverageManifest {
    /// Unique identifier for the run that produced this manifest.
    pub run_id: shiplog_ids::RunId,
    /// When the manifest was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// GitHub username the run targeted.
    pub user: String,
    /// Overall time window for the run.
    pub window: TimeWindow,
    /// "created" or "merged".
    pub mode: String,
    /// Source systems queried (e.g. `["github"]`).
    pub sources: Vec<String>,
    /// Per-window coverage slices with fetch statistics.
    pub slices: Vec<CoverageSlice>,
    /// Human-readable warnings encountered during the run.
    pub warnings: Vec<String>,
    /// Overall completeness verdict.
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

    #[test]
    fn completeness_serde_roundtrip() {
        for variant in [
            Completeness::Complete,
            Completeness::Partial,
            Completeness::Unknown,
        ] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: Completeness = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn coverage_slice_serde_roundtrip() {
        let slice = CoverageSlice {
            window: window(),
            query: "author:octocat is:merged".into(),
            total_count: 10,
            fetched: 10,
            incomplete_results: Some(false),
            notes: vec!["all fetched".into()],
        };
        let json = serde_json::to_string(&slice).unwrap();
        let back: CoverageSlice = serde_json::from_str(&json).unwrap();
        assert_eq!(slice, back);
    }

    #[test]
    fn coverage_manifest_serde_roundtrip() {
        let manifest = CoverageManifest {
            run_id: shiplog_ids::RunId("test-run".into()),
            generated_at: chrono::Utc::now(),
            user: "testuser".into(),
            window: window(),
            mode: "merged".into(),
            sources: vec!["github".into()],
            slices: vec![CoverageSlice {
                window: window(),
                query: "author:testuser".into(),
                total_count: 5,
                fetched: 5,
                incomplete_results: None,
                notes: vec![],
            }],
            warnings: vec!["test warning".into()],
            completeness: Completeness::Partial,
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let back: CoverageManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, back);
    }
}
