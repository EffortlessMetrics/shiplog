//! Issue tracking utilities for shiplog.
//!
//! Provides types and utilities for working with issue tracker data
//! from various sources like GitHub, Jira, Linear, etc.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents the status of an issue/tracker item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueStatus {
    Open,
    InProgress,
    InReview,
    Closed,
    Merged,
    WontFix,
}

impl fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IssueStatus::Open => write!(f, "open"),
            IssueStatus::InProgress => write!(f, "in_progress"),
            IssueStatus::InReview => write!(f, "in_review"),
            IssueStatus::Closed => write!(f, "closed"),
            IssueStatus::Merged => write!(f, "merged"),
            IssueStatus::WontFix => write!(f, "wont_fix"),
        }
    }
}

/// Represents the priority of an issue/tracker item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Low,
    None,
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "critical"),
            Priority::High => write!(f, "high"),
            Priority::Medium => write!(f, "medium"),
            Priority::Low => write!(f, "low"),
            Priority::None => write!(f, "none"),
        }
    }
}

/// A tracker item/issue representation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackerItem {
    /// Unique identifier for the tracker item (e.g., "PROJ-123")
    pub id: String,
    /// Title/summary of the issue
    pub title: String,
    /// Description (may be empty)
    pub description: String,
    /// Current status
    pub status: IssueStatus,
    /// Priority level
    pub priority: Priority,
    /// Source tracker type (github, jira, linear, etc.)
    pub source: String,
    /// URL to the issue
    pub url: Option<String>,
    /// Labels attached to the issue
    pub labels: Vec<String>,
    /// Assignee (if any)
    pub assignee: Option<String>,
}

impl TrackerItem {
    /// Create a new tracker item with required fields.
    pub fn new(id: impl Into<String>, title: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            description: String::new(),
            status: IssueStatus::Open,
            priority: Priority::None,
            source: source.into(),
            url: None,
            labels: Vec::new(),
            assignee: None,
        }
    }

    /// Check if this is an open issue.
    pub fn is_open(&self) -> bool {
        matches!(
            self.status,
            IssueStatus::Open | IssueStatus::InProgress | IssueStatus::InReview
        )
    }

    /// Check if this is a closed/resolved issue.
    pub fn is_closed(&self) -> bool {
        matches!(
            self.status,
            IssueStatus::Closed | IssueStatus::Merged | IssueStatus::WontFix
        )
    }

    /// Check if this issue has a specific label (case-insensitive).
    pub fn has_label(&self, label: &str) -> bool {
        self.labels
            .iter()
            .any(|l| l.to_lowercase() == label.to_lowercase())
    }
}

/// A collection of tracker items with filtering utilities.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrackerCollection {
    items: Vec<TrackerItem>,
}

impl TrackerCollection {
    /// Create a new empty collection.
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    /// Create a collection from a vector of items.
    pub fn from_items(items: Vec<TrackerItem>) -> Self {
        Self { items }
    }

    /// Add an item to the collection.
    pub fn push(&mut self, item: TrackerItem) {
        self.items.push(item);
    }

    /// Get all items.
    pub fn items(&self) -> &[TrackerItem] {
        &self.items
    }

    /// Get the count of items.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Filter items by status.
    pub fn filter_by_status(&self, status: IssueStatus) -> Vec<&TrackerItem> {
        self.items.iter().filter(|i| i.status == status).collect()
    }

    /// Filter items by source.
    pub fn filter_by_source(&self, source: &str) -> Vec<&TrackerItem> {
        self.items
            .iter()
            .filter(|i| i.source == source)
            .collect()
    }

    /// Get all open issues.
    pub fn open_issues(&self) -> Vec<&TrackerItem> {
        self.items.iter().filter(|i| i.is_open()).collect()
    }

    /// Get all closed issues.
    pub fn closed_issues(&self) -> Vec<&TrackerItem> {
        self.items.iter().filter(|i| i.is_closed()).collect()
    }
}

impl std::iter::FromIterator<TrackerItem> for TrackerCollection {
    fn from_iter<T: IntoIterator<Item = TrackerItem>>(iter: T) -> Self {
        Self::from_items(iter.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracker_item_creation() {
        let item = TrackerItem::new("PROJ-1", "Test issue", "jira");
        assert_eq!(item.id, "PROJ-1");
        assert_eq!(item.title, "Test issue");
        assert_eq!(item.source, "jira");
        assert_eq!(item.status, IssueStatus::Open);
    }

    #[test]
    fn tracker_item_is_open() {
        let mut item = TrackerItem::new("PROJ-1", "Test", "github");
        assert!(item.is_open());

        item.status = IssueStatus::InProgress;
        assert!(item.is_open());

        item.status = IssueStatus::Closed;
        assert!(!item.is_open());
    }

    #[test]
    fn tracker_item_is_closed() {
        let mut item = TrackerItem::new("PROJ-1", "Test", "github");
        assert!(!item.is_closed());

        item.status = IssueStatus::Closed;
        assert!(item.is_closed());

        item.status = IssueStatus::Merged;
        assert!(item.is_closed());
    }

    #[test]
    fn tracker_item_has_label() {
        let mut item = TrackerItem::new("PROJ-1", "Test", "github");
        item.labels = vec!["bug".to_string(), "urgent".to_string()];

        assert!(item.has_label("bug"));
        assert!(item.has_label("BUG"));
        assert!(!item.has_label("feature"));
    }

    #[test]
    fn tracker_collection_from_iter() {
        let items = vec![
            TrackerItem::new("1", "Issue 1", "github"),
            TrackerItem::new("2", "Issue 2", "jira"),
        ];
        let collection: TrackerCollection = items.into_iter().collect();
        assert_eq!(collection.len(), 2);
    }

    #[test]
    fn tracker_collection_filter_by_status() {
        let mut collection = TrackerCollection::new();
        collection.push(TrackerItem::new("1", "Issue 1", "github"));
        collection.items[0].status = IssueStatus::Open;

        collection.push(TrackerItem::new("2", "Issue 2", "github"));
        collection.items[1].status = IssueStatus::Closed;

        let open = collection.filter_by_status(IssueStatus::Open);
        assert_eq!(open.len(), 1);
    }

    #[test]
    fn tracker_collection_filter_by_source() {
        let mut collection = TrackerCollection::new();
        collection.push(TrackerItem::new("1", "Issue 1", "github"));
        collection.push(TrackerItem::new("2", "Issue 2", "jira"));

        let github_items = collection.filter_by_source("github");
        assert_eq!(github_items.len(), 1);
    }

    #[test]
    fn issue_status_display() {
        assert_eq!(format!("{}", IssueStatus::Open), "open");
        assert_eq!(format!("{}", IssueStatus::InProgress), "in_progress");
        assert_eq!(format!("{}", IssueStatus::Closed), "closed");
    }

    #[test]
    fn priority_display() {
        assert_eq!(format!("{}", Priority::Critical), "critical");
        assert_eq!(format!("{}", Priority::High), "high");
        assert_eq!(format!("{}", Priority::Low), "low");
    }
}
