//! Boundary value tests for workstream receipt policy.
//!
//! Covers exact transition points, at-limit behavior, and extreme inputs.

use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT,
    max_cluster_receipts_for_kind, should_include_cluster_receipt, should_render_receipt_at,
    truncate_cluster_receipts,
};

// ============================================================================
// Exact boundary transitions for review receipts
// ============================================================================

#[test]
fn review_included_at_count_zero() {
    assert!(should_include_cluster_receipt(&EventKind::Review, 0));
}

#[test]
fn review_included_at_one_below_limit() {
    assert!(should_include_cluster_receipt(
        &EventKind::Review,
        WORKSTREAM_RECEIPT_LIMIT_REVIEW - 1
    ));
}

#[test]
fn review_excluded_at_exact_limit() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Review,
        WORKSTREAM_RECEIPT_LIMIT_REVIEW
    ));
}

#[test]
fn review_excluded_at_one_above_limit() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Review,
        WORKSTREAM_RECEIPT_LIMIT_REVIEW + 1
    ));
}

#[test]
fn review_excluded_at_usize_max() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Review,
        usize::MAX
    ));
}

// ============================================================================
// Exact boundary transitions for manual receipts
// ============================================================================

#[test]
fn manual_included_at_count_zero() {
    assert!(should_include_cluster_receipt(&EventKind::Manual, 0));
}

#[test]
fn manual_included_at_one_below_limit() {
    assert!(should_include_cluster_receipt(
        &EventKind::Manual,
        WORKSTREAM_RECEIPT_LIMIT_MANUAL - 1
    ));
}

#[test]
fn manual_excluded_at_exact_limit() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Manual,
        WORKSTREAM_RECEIPT_LIMIT_MANUAL
    ));
}

#[test]
fn manual_excluded_at_one_above_limit() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Manual,
        WORKSTREAM_RECEIPT_LIMIT_MANUAL + 1
    ));
}

#[test]
fn manual_excluded_at_usize_max() {
    assert!(!should_include_cluster_receipt(
        &EventKind::Manual,
        usize::MAX
    ));
}

// ============================================================================
// PR receipts: unbounded boundary
// ============================================================================

#[test]
fn pr_included_at_zero() {
    assert!(should_include_cluster_receipt(&EventKind::PullRequest, 0));
}

#[test]
fn pr_included_at_total_limit() {
    assert!(should_include_cluster_receipt(
        &EventKind::PullRequest,
        WORKSTREAM_RECEIPT_LIMIT_TOTAL
    ));
}

#[test]
fn pr_included_at_usize_max_minus_one() {
    assert!(should_include_cluster_receipt(
        &EventKind::PullRequest,
        usize::MAX - 1
    ));
}

// ============================================================================
// Render receipt boundary transitions
// ============================================================================

#[test]
fn render_visible_at_index_zero() {
    assert!(should_render_receipt_at(0));
}

#[test]
fn render_visible_at_one_below_limit() {
    assert!(should_render_receipt_at(WORKSTREAM_RECEIPT_RENDER_LIMIT - 1));
}

#[test]
fn render_hidden_at_exact_limit() {
    assert!(!should_render_receipt_at(WORKSTREAM_RECEIPT_RENDER_LIMIT));
}

#[test]
fn render_hidden_at_one_above_limit() {
    assert!(!should_render_receipt_at(
        WORKSTREAM_RECEIPT_RENDER_LIMIT + 1
    ));
}

#[test]
fn render_hidden_at_usize_max() {
    assert!(!should_render_receipt_at(usize::MAX));
}

// ============================================================================
// Truncation at exact limits
// ============================================================================

#[test]
fn truncate_exactly_at_limit() {
    let mut v: Vec<usize> = (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(v, (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect::<Vec<_>>());
}

#[test]
fn truncate_one_over_limit() {
    let mut v: Vec<usize> = (0..=WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(*v.last().unwrap(), WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1);
}

#[test]
fn truncate_one_under_limit() {
    let original_len = WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1;
    let mut v: Vec<usize> = (0..original_len).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), original_len);
}

#[test]
fn truncate_empty() {
    let mut v: Vec<usize> = vec![];
    truncate_cluster_receipts(&mut v);
    assert!(v.is_empty());
}

#[test]
fn truncate_single_element() {
    let mut v = vec![42usize];
    truncate_cluster_receipts(&mut v);
    assert_eq!(v, vec![42]);
}

#[test]
fn truncate_very_large_vec_preserves_order() {
    let mut v: Vec<usize> = (0..100_000).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    for (i, val) in v.iter().enumerate() {
        assert_eq!(*val, i);
    }
}

#[test]
fn truncate_idempotent() {
    let mut v: Vec<usize> = (0..50).collect();
    truncate_cluster_receipts(&mut v);
    let after_first = v.clone();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v, after_first);
}

// ============================================================================
// Mixed-kind accumulation at exact boundaries
// ============================================================================

#[test]
fn accumulate_reviews_exactly_at_limit() {
    let mut count = 0;
    let mut accepted = 0;
    for _ in 0..(WORKSTREAM_RECEIPT_LIMIT_REVIEW + 2) {
        if should_include_cluster_receipt(&EventKind::Review, count) {
            accepted += 1;
            count += 1;
        }
    }
    assert_eq!(accepted, WORKSTREAM_RECEIPT_LIMIT_REVIEW);
}

#[test]
fn accumulate_manual_exactly_at_limit() {
    let mut count = 0;
    let mut accepted = 0;
    for _ in 0..(WORKSTREAM_RECEIPT_LIMIT_MANUAL + 2) {
        if should_include_cluster_receipt(&EventKind::Manual, count) {
            accepted += 1;
            count += 1;
        }
    }
    assert_eq!(accepted, WORKSTREAM_RECEIPT_LIMIT_MANUAL);
}

#[test]
fn accumulate_mixed_up_to_total_then_truncate() {
    // Fill with PRs up to twice the total limit, then truncate
    let mut receipts: Vec<String> = (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL * 2)
        .map(|i| format!("pr-{i}"))
        .collect();
    truncate_cluster_receipts(&mut receipts);
    assert_eq!(receipts.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(receipts[0], "pr-0");
    assert_eq!(
        receipts[WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1],
        format!("pr-{}", WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1)
    );
}

// ============================================================================
// max_cluster_receipts_for_kind boundaries
// ============================================================================

#[test]
fn max_for_pr_is_usize_max() {
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::PullRequest),
        usize::MAX
    );
}

#[test]
fn max_for_review_equals_constant() {
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::Review),
        WORKSTREAM_RECEIPT_LIMIT_REVIEW
    );
}

#[test]
fn max_for_manual_equals_constant() {
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::Manual),
        WORKSTREAM_RECEIPT_LIMIT_MANUAL
    );
}

// ============================================================================
// Render limit vs cluster limit interaction
// ============================================================================

#[test]
fn render_limit_does_not_exceed_total_limit() {
    const { assert!(WORKSTREAM_RECEIPT_RENDER_LIMIT <= WORKSTREAM_RECEIPT_LIMIT_TOTAL) };
}

#[test]
fn items_at_render_boundary_but_within_total() {
    // Index at render limit is hidden but within total cluster limit
    let idx = WORKSTREAM_RECEIPT_RENDER_LIMIT;
    assert!(!should_render_receipt_at(idx));
    assert!(idx < WORKSTREAM_RECEIPT_LIMIT_TOTAL);
}

#[test]
fn all_render_visible_indices_within_total() {
    for i in 0..WORKSTREAM_RECEIPT_RENDER_LIMIT {
        assert!(should_render_receipt_at(i));
        assert!(i < WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    }
}
