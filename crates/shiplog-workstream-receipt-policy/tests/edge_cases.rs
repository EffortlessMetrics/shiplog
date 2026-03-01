//! Edge-case tests for shiplog-workstream-receipt-policy.
//!
//! Covers receipt validation edge cases, boundary conditions, mixed-kind
//! accumulation simulations, and policy enforcement across all event kinds.

use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT, max_cluster_receipts_for_kind,
    should_include_cluster_receipt, should_render_receipt_at, truncate_cluster_receipts,
};

// ===========================================================================
// Boundary conditions at zero
// ===========================================================================

#[test]
fn all_kinds_included_at_zero_count() {
    assert!(should_include_cluster_receipt(&EventKind::PullRequest, 0));
    assert!(should_include_cluster_receipt(&EventKind::Review, 0));
    assert!(should_include_cluster_receipt(&EventKind::Manual, 0));
}

#[test]
fn render_at_index_zero_is_always_visible() {
    assert!(should_render_receipt_at(0));
}

#[test]
fn truncate_empty_vec_is_noop_for_all_types() {
    let mut strings: Vec<String> = vec![];
    truncate_cluster_receipts(&mut strings);
    assert!(strings.is_empty());

    let mut ints: Vec<i64> = vec![];
    truncate_cluster_receipts(&mut ints);
    assert!(ints.is_empty());

    let mut bools: Vec<bool> = vec![];
    truncate_cluster_receipts(&mut bools);
    assert!(bools.is_empty());
}

// ===========================================================================
// Exact boundary transitions
// ===========================================================================

#[test]
fn review_boundary_at_limit_minus_one_and_limit() {
    let limit = WORKSTREAM_RECEIPT_LIMIT_REVIEW;
    assert!(should_include_cluster_receipt(
        &EventKind::Review,
        limit - 1
    ));
    assert!(!should_include_cluster_receipt(&EventKind::Review, limit));
    assert!(!should_include_cluster_receipt(
        &EventKind::Review,
        limit + 1
    ));
}

#[test]
fn manual_boundary_at_limit_minus_one_and_limit() {
    let limit = WORKSTREAM_RECEIPT_LIMIT_MANUAL;
    assert!(should_include_cluster_receipt(
        &EventKind::Manual,
        limit - 1
    ));
    assert!(!should_include_cluster_receipt(&EventKind::Manual, limit));
    assert!(!should_include_cluster_receipt(
        &EventKind::Manual,
        limit + 1
    ));
}

#[test]
fn render_boundary_at_limit_minus_one_and_limit() {
    let limit = WORKSTREAM_RECEIPT_RENDER_LIMIT;
    assert!(should_render_receipt_at(limit - 1));
    assert!(!should_render_receipt_at(limit));
    assert!(!should_render_receipt_at(limit + 1));
}

#[test]
fn pr_included_at_very_large_count() {
    assert!(should_include_cluster_receipt(
        &EventKind::PullRequest,
        1_000_000
    ));
    assert!(should_include_cluster_receipt(
        &EventKind::PullRequest,
        usize::MAX - 1
    ));
}

// ===========================================================================
// Truncation edge cases
// ===========================================================================

#[test]
fn truncate_at_exactly_total_limit_preserves_all() {
    let mut v: Vec<usize> = (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect();
    let original = v.clone();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v, original);
}

#[test]
fn truncate_one_above_limit_removes_last() {
    let mut v: Vec<usize> = (0..=WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(*v.last().unwrap(), WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1);
}

#[test]
fn truncate_one_below_limit_preserves_all() {
    let len = WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1;
    let mut v: Vec<usize> = (0..len).collect();
    let original = v.clone();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v, original);
}

#[test]
fn truncate_single_element() {
    let mut v = vec![42u32];
    truncate_cluster_receipts(&mut v);
    assert_eq!(v, vec![42]);
}

#[test]
fn truncate_very_large_vec() {
    let mut v: Vec<usize> = (0..10_000).collect();
    truncate_cluster_receipts(&mut v);
    assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(v[0], 0);
    assert_eq!(*v.last().unwrap(), WORKSTREAM_RECEIPT_LIMIT_TOTAL - 1);
}

// ===========================================================================
// Mixed-kind accumulation simulation
// ===========================================================================

#[test]
fn mixed_pr_then_review_accumulation() {
    let kinds = [
        EventKind::PullRequest,
        EventKind::PullRequest,
        EventKind::PullRequest,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review, // 6th review
    ];

    let mut count = 0;
    let mut accepted_kinds = vec![];
    for kind in &kinds {
        if should_include_cluster_receipt(kind, count) {
            accepted_kinds.push(kind.clone());
            count += 1;
        }
    }

    let pr_accepted = accepted_kinds
        .iter()
        .filter(|k| **k == EventKind::PullRequest)
        .count();
    let review_accepted = accepted_kinds
        .iter()
        .filter(|k| **k == EventKind::Review)
        .count();

    // All PRs accepted (unbounded per-kind limit)
    assert_eq!(pr_accepted, 3);
    // Reviews capped: after 3 PRs, count=3; reviews at count 3,4 accepted (3<5, 4<5),
    // review at count 5 rejected (5 < 5 = false)
    assert_eq!(review_accepted, 2);
}

#[test]
fn mixed_all_kinds_accumulation_respects_per_kind_limits() {
    // Simulate: 3 reviews, 3 manuals, 3 PRs, 3 more reviews
    let kinds = [
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
        EventKind::Manual,
        EventKind::Manual,
        EventKind::Manual,
        EventKind::PullRequest,
        EventKind::PullRequest,
        EventKind::PullRequest,
        EventKind::Review,
        EventKind::Review,
        EventKind::Review,
    ];

    let mut count = 0;
    let mut accepted = vec![];
    for kind in &kinds {
        if should_include_cluster_receipt(kind, count) {
            accepted.push(kind.clone());
            count += 1;
        }
    }

    // All items check total count against per-kind limit:
    // Review at 0: 0<5 yes, count=1
    // Review at 1: 1<5 yes, count=2
    // Review at 2: 2<5 yes, count=3
    // Manual at 3: 3<7 yes, count=4
    // Manual at 4: 4<7 yes, count=5
    // Manual at 5: 5<7 yes, count=6
    // PR at 6: 6<MAX yes, count=7
    // PR at 7: 7<MAX yes, count=8
    // PR at 8: 8<MAX yes, count=9
    // Review at 9: 9<5 = false, rejected
    // Review at 9: still 9<5 = false, rejected
    // Review at 9: still 9<5 = false, rejected
    let review_accepted = accepted.iter().filter(|k| **k == EventKind::Review).count();
    let manual_accepted = accepted.iter().filter(|k| **k == EventKind::Manual).count();
    let pr_accepted = accepted
        .iter()
        .filter(|k| **k == EventKind::PullRequest)
        .count();

    assert_eq!(review_accepted, 3);
    assert_eq!(manual_accepted, 3);
    assert_eq!(pr_accepted, 3);
    assert_eq!(accepted.len(), 9);
}

#[test]
fn truncation_after_mixed_accumulation() {
    // Simulate accepting 15 PRs, then truncating
    let mut receipts: Vec<String> = (0..15).map(|i| format!("receipt-{i}")).collect();
    truncate_cluster_receipts(&mut receipts);

    assert_eq!(receipts.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    assert_eq!(receipts[0], "receipt-0");
    assert_eq!(receipts[9], "receipt-9");
}

// ===========================================================================
// Policy constants relationships
// ===========================================================================

#[test]
fn render_limit_fits_within_total_limit() {
    const { assert!(WORKSTREAM_RECEIPT_RENDER_LIMIT <= WORKSTREAM_RECEIPT_LIMIT_TOTAL) }
}

#[test]
fn review_limit_fits_within_total_limit() {
    const { assert!(WORKSTREAM_RECEIPT_LIMIT_REVIEW <= WORKSTREAM_RECEIPT_LIMIT_TOTAL) }
}

#[test]
fn manual_limit_fits_within_total_limit() {
    const { assert!(WORKSTREAM_RECEIPT_LIMIT_MANUAL <= WORKSTREAM_RECEIPT_LIMIT_TOTAL) }
}

#[test]
fn review_limit_less_or_equal_manual_limit() {
    const { assert!(WORKSTREAM_RECEIPT_LIMIT_REVIEW <= WORKSTREAM_RECEIPT_LIMIT_MANUAL) }
}

#[test]
fn render_limit_less_or_equal_review_limit() {
    const { assert!(WORKSTREAM_RECEIPT_RENDER_LIMIT <= WORKSTREAM_RECEIPT_LIMIT_REVIEW) }
}

// ===========================================================================
// Full visibility sweep
// ===========================================================================

#[test]
fn render_visibility_exact_count() {
    let visible: usize = (0..100).filter(|i| should_render_receipt_at(*i)).count();
    assert_eq!(visible, WORKSTREAM_RECEIPT_RENDER_LIMIT);
}

#[test]
fn all_rendered_indices_are_contiguous_from_zero() {
    let visible_indices: Vec<usize> = (0..100).filter(|i| should_render_receipt_at(*i)).collect();
    let expected: Vec<usize> = (0..WORKSTREAM_RECEIPT_RENDER_LIMIT).collect();
    assert_eq!(visible_indices, expected);
}

// ===========================================================================
// max_cluster_receipts_for_kind exhaustive check
// ===========================================================================

#[test]
fn max_receipts_per_kind_values() {
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::PullRequest),
        usize::MAX
    );
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::Review),
        WORKSTREAM_RECEIPT_LIMIT_REVIEW
    );
    assert_eq!(
        max_cluster_receipts_for_kind(&EventKind::Manual),
        WORKSTREAM_RECEIPT_LIMIT_MANUAL
    );
}

#[test]
fn pr_max_is_strictly_greater_than_review_and_manual() {
    let pr = max_cluster_receipts_for_kind(&EventKind::PullRequest);
    let review = max_cluster_receipts_for_kind(&EventKind::Review);
    let manual = max_cluster_receipts_for_kind(&EventKind::Manual);

    assert!(pr > review);
    assert!(pr > manual);
}
