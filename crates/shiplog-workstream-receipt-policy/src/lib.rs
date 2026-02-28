//! Shared policy for receipt selection and display limits.
//!
//! This crate is intentionally tiny and focused: it codifies the policy constants
//! used by workstream clustering and markdown rendering so they stay in sync.

use shiplog_schema::event::EventKind;

/// Max receipts kept during clustering for Review events.
pub const WORKSTREAM_RECEIPT_LIMIT_REVIEW: usize = 5;

/// Max receipts kept during clustering for Manual events.
pub const WORKSTREAM_RECEIPT_LIMIT_MANUAL: usize = 7;

/// Hard total limit for receipts kept on a workstream after clustering.
pub const WORKSTREAM_RECEIPT_LIMIT_TOTAL: usize = 10;

/// Max receipts shown in the main packet section per workstream.
pub const WORKSTREAM_RECEIPT_RENDER_LIMIT: usize = 5;

/// Clamp an in-memory receipt vector to the total cluster cap.
pub fn truncate_cluster_receipts<T>(receipts: &mut Vec<T>) {
    receipts.truncate(WORKSTREAM_RECEIPT_LIMIT_TOTAL);
}

/// Capacity for receipts from a specific event kind before capping.
#[must_use]
pub fn max_cluster_receipts_for_kind(kind: &EventKind) -> usize {
    match kind {
        EventKind::PullRequest => usize::MAX,
        EventKind::Review => WORKSTREAM_RECEIPT_LIMIT_REVIEW,
        EventKind::Manual => WORKSTREAM_RECEIPT_LIMIT_MANUAL,
    }
}

/// Whether one more event of this kind should be recorded into receipts.
#[must_use]
pub fn should_include_cluster_receipt(kind: &EventKind, current_count: usize) -> bool {
    current_count < max_cluster_receipts_for_kind(kind)
}

/// Whether this zero-based index should be shown in the packet main receipts section.
#[must_use]
pub fn should_render_receipt_at(index: usize) -> bool {
    index < WORKSTREAM_RECEIPT_RENDER_LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn review_receipt_limit_is_five() {
        assert_eq!(WORKSTREAM_RECEIPT_LIMIT_REVIEW, 5);
    }

    #[test]
    fn manual_receipt_limit_is_seven() {
        assert_eq!(WORKSTREAM_RECEIPT_LIMIT_MANUAL, 7);
    }

    #[test]
    fn total_receipt_limit_is_ten() {
        assert_eq!(WORKSTREAM_RECEIPT_LIMIT_TOTAL, 10);
    }

    #[test]
    fn render_limit_is_five() {
        assert_eq!(WORKSTREAM_RECEIPT_RENDER_LIMIT, 5);
    }

    #[test]
    fn pull_requests_are_unbounded_for_cluster_receipts() {
        assert!(should_include_cluster_receipt(
            &EventKind::PullRequest,
            usize::MAX - 1
        ));
    }

    #[test]
    fn review_limit_is_exclusive() {
        assert!(should_include_cluster_receipt(&EventKind::Review, 0));
        assert!(should_include_cluster_receipt(&EventKind::Review, 4));
        assert!(!should_include_cluster_receipt(&EventKind::Review, 5));
    }

    #[test]
    fn manual_limit_is_exclusive() {
        assert!(should_include_cluster_receipt(&EventKind::Manual, 0));
        assert!(should_include_cluster_receipt(&EventKind::Manual, 6));
        assert!(!should_include_cluster_receipt(&EventKind::Manual, 7));
    }

    #[test]
    fn render_window_is_top_five_only() {
        assert!(should_render_receipt_at(0));
        assert!(should_render_receipt_at(4));
        assert!(!should_render_receipt_at(5));
    }

    #[test]
    fn truncate_empty_vec_is_noop() {
        let mut v: Vec<i32> = vec![];
        truncate_cluster_receipts(&mut v);
        assert!(v.is_empty());
    }

    #[test]
    fn truncate_at_exactly_limit_preserves_all() {
        let mut v: Vec<usize> = (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect();
        truncate_cluster_receipts(&mut v);
        assert_eq!(v.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        assert_eq!(v, (0..WORKSTREAM_RECEIPT_LIMIT_TOTAL).collect::<Vec<_>>());
    }

    #[test]
    fn truncate_below_limit_preserves_all() {
        let mut v: Vec<usize> = (0..3).collect();
        truncate_cluster_receipts(&mut v);
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn max_cluster_receipts_for_pr_is_usize_max() {
        assert_eq!(
            max_cluster_receipts_for_kind(&EventKind::PullRequest),
            usize::MAX
        );
    }

    #[test]
    fn max_cluster_receipts_for_review() {
        assert_eq!(
            max_cluster_receipts_for_kind(&EventKind::Review),
            WORKSTREAM_RECEIPT_LIMIT_REVIEW
        );
    }

    #[test]
    fn max_cluster_receipts_for_manual() {
        assert_eq!(
            max_cluster_receipts_for_kind(&EventKind::Manual),
            WORKSTREAM_RECEIPT_LIMIT_MANUAL
        );
    }

    #[test]
    fn review_and_manual_limits_both_fit_within_total() {
        const {
            assert!(WORKSTREAM_RECEIPT_LIMIT_REVIEW <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);
            assert!(WORKSTREAM_RECEIPT_LIMIT_MANUAL <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        }
    }

    #[test]
    fn render_limit_within_total_limit() {
        const {
            assert!(WORKSTREAM_RECEIPT_RENDER_LIMIT <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        }
    }

    #[test]
    fn once_excluded_always_excluded_for_reviews() {
        let boundary = WORKSTREAM_RECEIPT_LIMIT_REVIEW;
        assert!(!should_include_cluster_receipt(&EventKind::Review, boundary));
        assert!(!should_include_cluster_receipt(
            &EventKind::Review,
            boundary + 1
        ));
        assert!(!should_include_cluster_receipt(
            &EventKind::Review,
            boundary + 100
        ));
    }

    #[test]
    fn once_excluded_always_excluded_for_manual() {
        let boundary = WORKSTREAM_RECEIPT_LIMIT_MANUAL;
        assert!(!should_include_cluster_receipt(&EventKind::Manual, boundary));
        assert!(!should_include_cluster_receipt(
            &EventKind::Manual,
            boundary + 1
        ));
        assert!(!should_include_cluster_receipt(
            &EventKind::Manual,
            boundary + 100
        ));
    }

    proptest! {
        #[test]
        fn prop_cluster_receipt_limits_stay_consistent(kind_code in 0u8..3, count in 0usize..30) {
            let kind = match kind_code {
                0 => EventKind::PullRequest,
                1 => EventKind::Review,
                _ => EventKind::Manual,
            };

            let allowed = should_include_cluster_receipt(&kind, count);
            let max = max_cluster_receipts_for_kind(&kind);
            let expected = count < max;

            prop_assert_eq!(allowed, expected);
        }

        #[test]
        fn prop_render_visibility_matches_limit(index in 0usize..20) {
            let visible = should_render_receipt_at(index);
            let expected = index < WORKSTREAM_RECEIPT_RENDER_LIMIT;
            prop_assert_eq!(visible, expected);
        }

        #[test]
        fn prop_inclusion_is_monotonically_decreasing(kind_code in 0u8..3, count in 0usize..100) {
            let kind = match kind_code {
                0 => EventKind::PullRequest,
                1 => EventKind::Review,
                _ => EventKind::Manual,
            };

            if !should_include_cluster_receipt(&kind, count) {
                // Once excluded, must stay excluded for all higher counts
                prop_assert!(!should_include_cluster_receipt(&kind, count + 1));
            }
        }
    }
}
