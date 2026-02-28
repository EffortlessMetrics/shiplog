// Integration-style checks for the workstream receipt policy crate.

use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT, max_cluster_receipts_for_kind,
    should_include_cluster_receipt, should_render_receipt_at, truncate_cluster_receipts,
};

#[test]
fn policy_constants_are_consistent_for_receipt_strategy() {
    assert_eq!(WORKSTREAM_RECEIPT_LIMIT_REVIEW, 5);
    assert_eq!(WORKSTREAM_RECEIPT_LIMIT_MANUAL, 7);
    assert_eq!(WORKSTREAM_RECEIPT_LIMIT_TOTAL, 10);
    assert_eq!(WORKSTREAM_RECEIPT_RENDER_LIMIT, 5);
}

#[test]
fn policy_render_cap_captures_expected_visibility_window() {
    let visible_count = (0..20).filter(|idx| should_render_receipt_at(*idx)).count();
    assert_eq!(visible_count, WORKSTREAM_RECEIPT_RENDER_LIMIT);
}

#[test]
fn policy_total_cap_truncation_is_stable() {
    let mut data = (0..20).collect::<Vec<usize>>();
    truncate_cluster_receipts(&mut data);
    assert_eq!(data.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    truncate_cluster_receipts(&mut data);
    assert_eq!(data.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
}

#[test]
fn policy_truncation_preserves_order() {
    let mut data: Vec<usize> = (0..20).collect();
    truncate_cluster_receipts(&mut data);
    for (i, val) in data.iter().enumerate() {
        assert_eq!(*val, i);
    }
}

#[test]
fn policy_kind_limits_are_hierarchical() {
    let pr_max = max_cluster_receipts_for_kind(&EventKind::PullRequest);
    let review_max = max_cluster_receipts_for_kind(&EventKind::Review);
    let manual_max = max_cluster_receipts_for_kind(&EventKind::Manual);

    assert!(pr_max > manual_max);
    assert!(manual_max > review_max);
}

#[test]
fn policy_simulated_mixed_stream_respects_caps() {
    // Simulate how the clusterer would accumulate receipts.
    // Note: should_include_cluster_receipt compares total receipt count
    // against the per-kind cap, so a PR in slot 0 consumes a slot.
    let kinds = [
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review, // 6th review, total count=5 so 5 < 5 = false
        EventKind::Manual,
        EventKind::PullRequest,
    ];

    let mut receipt_count = 0;
    let mut accepted = vec![];
    for kind in &kinds {
        if should_include_cluster_receipt(kind, receipt_count) {
            accepted.push(kind);
            receipt_count += 1;
        }
    }

    // First 5 reviews accepted, 6th rejected (total count=5, 5 < 5 = false)
    let review_count = accepted
        .iter()
        .filter(|k| ***k == EventKind::Review)
        .count();
    assert_eq!(review_count, WORKSTREAM_RECEIPT_LIMIT_REVIEW);
}

#[test]
fn policy_render_window_starts_at_zero() {
    assert!(should_render_receipt_at(0));
}

#[test]
fn policy_render_exactly_at_boundary() {
    assert!(should_render_receipt_at(
        WORKSTREAM_RECEIPT_RENDER_LIMIT - 1
    ));
    assert!(!should_render_receipt_at(WORKSTREAM_RECEIPT_RENDER_LIMIT));
}

#[test]
fn policy_truncation_with_strings() {
    let mut data: Vec<String> = (0..15).map(|i| format!("receipt-{i}")).collect();
    truncate_cluster_receipts(&mut data);
    assert_eq!(data.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(data[0], "receipt-0");
    assert_eq!(data[9], "receipt-9");
}
