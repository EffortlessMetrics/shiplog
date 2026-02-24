// Integration-style checks for the workstream receipt policy crate.

use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT, should_render_receipt_at,
    truncate_cluster_receipts,
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
