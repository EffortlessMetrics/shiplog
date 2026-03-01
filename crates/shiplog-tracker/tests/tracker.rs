use proptest::prelude::*;
use shiplog_tracker::*;

// ── IssueStatus ─────────────────────────────────────────────────────

#[test]
fn issue_status_display_all_variants() {
    let cases = vec![
        (IssueStatus::Open, "open"),
        (IssueStatus::InProgress, "in_progress"),
        (IssueStatus::InReview, "in_review"),
        (IssueStatus::Closed, "closed"),
        (IssueStatus::Merged, "merged"),
        (IssueStatus::WontFix, "wont_fix"),
    ];
    for (status, expected) in cases {
        assert_eq!(format!("{status}"), expected);
    }
}

// ── Priority ────────────────────────────────────────────────────────

#[test]
fn priority_display_all_variants() {
    let cases = vec![
        (Priority::Critical, "critical"),
        (Priority::High, "high"),
        (Priority::Medium, "medium"),
        (Priority::Low, "low"),
        (Priority::None, "none"),
    ];
    for (p, expected) in cases {
        assert_eq!(format!("{p}"), expected);
    }
}

// ── TrackerItem ─────────────────────────────────────────────────────

#[test]
fn new_item_defaults() {
    let item = TrackerItem::new("ID-1", "Title", "github");
    assert_eq!(item.id, "ID-1");
    assert_eq!(item.title, "Title");
    assert_eq!(item.source, "github");
    assert!(item.description.is_empty());
    assert_eq!(item.status, IssueStatus::Open);
    assert_eq!(item.priority, Priority::None);
    assert!(item.url.is_none());
    assert!(item.labels.is_empty());
    assert!(item.assignee.is_none());
}

#[test]
fn is_open_covers_all_open_statuses() {
    let open = [
        IssueStatus::Open,
        IssueStatus::InProgress,
        IssueStatus::InReview,
    ];
    let closed = [
        IssueStatus::Closed,
        IssueStatus::Merged,
        IssueStatus::WontFix,
    ];
    for s in open {
        let mut item = TrackerItem::new("1", "t", "s");
        item.status = s;
        assert!(item.is_open(), "expected is_open for {:?}", item.status);
        assert!(!item.is_closed());
    }
    for s in closed {
        let mut item = TrackerItem::new("1", "t", "s");
        item.status = s;
        assert!(item.is_closed(), "expected is_closed for {:?}", item.status);
        assert!(!item.is_open());
    }
}

#[test]
fn has_label_case_insensitive() {
    let mut item = TrackerItem::new("1", "t", "s");
    item.labels = vec!["Bug".into(), "URGENT".into()];
    assert!(item.has_label("bug"));
    assert!(item.has_label("BUG"));
    assert!(item.has_label("Bug"));
    assert!(item.has_label("urgent"));
    assert!(!item.has_label("feature"));
}

#[test]
fn has_label_empty_labels() {
    let item = TrackerItem::new("1", "t", "s");
    assert!(!item.has_label("anything"));
}

proptest! {
    #[test]
    fn new_item_preserves_fields(
        id in "[a-zA-Z0-9-]{1,20}",
        title in ".{1,40}",
        source in "(github|jira|linear)"
    ) {
        let item = TrackerItem::new(&id, &title, &source);
        prop_assert_eq!(&item.id, &id);
        prop_assert_eq!(&item.title, &title);
        prop_assert_eq!(&item.source, &source);
    }
}

// ── TrackerCollection ───────────────────────────────────────────────

#[test]
fn collection_new_is_empty() {
    let c = TrackerCollection::new();
    assert!(c.is_empty());
    assert_eq!(c.len(), 0);
}

#[test]
fn collection_from_items() {
    let items = vec![
        TrackerItem::new("1", "A", "github"),
        TrackerItem::new("2", "B", "jira"),
    ];
    let c = TrackerCollection::from_items(items);
    assert_eq!(c.len(), 2);
}

#[test]
fn collection_from_iter() {
    let c: TrackerCollection = (0..5)
        .map(|i| TrackerItem::new(format!("{i}"), "t", "src"))
        .collect();
    assert_eq!(c.len(), 5);
}

#[test]
fn filter_by_status_returns_matching() {
    let mut c = TrackerCollection::new();
    let mut open = TrackerItem::new("1", "open", "gh");
    open.status = IssueStatus::Open;
    let mut closed = TrackerItem::new("2", "closed", "gh");
    closed.status = IssueStatus::Closed;
    c.push(open);
    c.push(closed);
    assert_eq!(c.filter_by_status(IssueStatus::Open).len(), 1);
    assert_eq!(c.filter_by_status(IssueStatus::Closed).len(), 1);
    assert_eq!(c.filter_by_status(IssueStatus::Merged).len(), 0);
}

#[test]
fn filter_by_source_returns_matching() {
    let mut c = TrackerCollection::new();
    c.push(TrackerItem::new("1", "t", "github"));
    c.push(TrackerItem::new("2", "t", "jira"));
    c.push(TrackerItem::new("3", "t", "github"));
    assert_eq!(c.filter_by_source("github").len(), 2);
    assert_eq!(c.filter_by_source("jira").len(), 1);
    assert_eq!(c.filter_by_source("linear").len(), 0);
}

#[test]
fn open_and_closed_partition() {
    let mut c = TrackerCollection::new();
    let statuses = [
        IssueStatus::Open,
        IssueStatus::InProgress,
        IssueStatus::Closed,
        IssueStatus::Merged,
        IssueStatus::WontFix,
    ];
    for (i, s) in statuses.iter().enumerate() {
        let mut item = TrackerItem::new(format!("{i}"), "t", "s");
        item.status = s.clone();
        c.push(item);
    }
    let open = c.open_issues();
    let closed = c.closed_issues();
    assert_eq!(open.len() + closed.len(), c.len());
}

// ── Serde roundtrip ─────────────────────────────────────────────────

#[test]
fn tracker_item_serde_roundtrip() {
    let mut item = TrackerItem::new("PROJ-42", "Fix the bug", "github");
    item.labels = vec!["bug".into()];
    item.assignee = Some("alice".into());
    item.url = Some("https://example.com".into());
    item.status = IssueStatus::InProgress;
    item.priority = Priority::High;

    let json = serde_json::to_string(&item).unwrap();
    let deserialized: TrackerItem = serde_json::from_str(&json).unwrap();
    assert_eq!(item, deserialized);
}

#[test]
fn issue_status_serde_roundtrip() {
    let statuses = vec![
        IssueStatus::Open,
        IssueStatus::InProgress,
        IssueStatus::InReview,
        IssueStatus::Closed,
        IssueStatus::Merged,
        IssueStatus::WontFix,
    ];
    for s in statuses {
        let json = serde_json::to_string(&s).unwrap();
        let back: IssueStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(s, back);
    }
}
