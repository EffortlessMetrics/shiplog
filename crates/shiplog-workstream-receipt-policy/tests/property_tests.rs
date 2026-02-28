// Property tests for workstream receipt policy functions.

use proptest::prelude::*;
use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT, max_cluster_receipts_for_kind,
    should_include_cluster_receipt, should_render_receipt_at, truncate_cluster_receipts,
};

proptest! {
    #[test]
    fn prop_cluster_receipt_boundary_is_kind_specific(kind_code in 0u8..3, count in 0usize..64) {
        let kind = match kind_code {
            0 => EventKind::PullRequest,
            1 => EventKind::Review,
            _ => EventKind::Manual,
        };

        let included = should_include_cluster_receipt(&kind, count);
        let expected = match kind {
            EventKind::PullRequest => true,
            EventKind::Review => count < WORKSTREAM_RECEIPT_LIMIT_REVIEW,
            EventKind::Manual => count < WORKSTREAM_RECEIPT_LIMIT_MANUAL,
        };

        prop_assert_eq!(included, expected);
    }

    #[test]
    fn prop_render_receipt_visibility_matches_render_limit(index in 0usize..64) {
        prop_assert_eq!(should_render_receipt_at(index), index < WORKSTREAM_RECEIPT_RENDER_LIMIT);
    }

    #[test]
    fn prop_total_receipt_truncation_keeps_cap(len in 0usize..128) {
        let mut receipts = (0..len).collect::<Vec<usize>>();
        truncate_cluster_receipts(&mut receipts);

        prop_assert!(receipts.len() <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        if len > WORKSTREAM_RECEIPT_LIMIT_TOTAL {
            prop_assert_eq!(receipts.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
        }
    }

    #[test]
    fn prop_truncation_is_idempotent(len in 0usize..128) {
        let mut receipts = (0..len).collect::<Vec<usize>>();
        truncate_cluster_receipts(&mut receipts);
        let after_first = receipts.clone();
        truncate_cluster_receipts(&mut receipts);
        prop_assert_eq!(receipts, after_first);
    }

    #[test]
    fn prop_truncation_preserves_prefix(len in 0usize..128) {
        let original: Vec<usize> = (0..len).collect();
        let mut receipts = original.clone();
        truncate_cluster_receipts(&mut receipts);

        // The truncated vector should be a prefix of the original
        for (i, val) in receipts.iter().enumerate() {
            prop_assert_eq!(*val, original[i]);
        }
    }

    #[test]
    fn prop_inclusion_monotonicity(kind_code in 0u8..3, count in 0usize..100) {
        let kind = match kind_code {
            0 => EventKind::PullRequest,
            1 => EventKind::Review,
            _ => EventKind::Manual,
        };

        // If excluded at count, must be excluded at count+1
        if !should_include_cluster_receipt(&kind, count) {
            prop_assert!(!should_include_cluster_receipt(&kind, count + 1));
        }

        // If included at count+1, must be included at count
        if should_include_cluster_receipt(&kind, count + 1) {
            prop_assert!(should_include_cluster_receipt(&kind, count));
        }
    }

    #[test]
    fn prop_max_receipts_is_consistent_with_should_include(kind_code in 0u8..3) {
        let kind = match kind_code {
            0 => EventKind::PullRequest,
            1 => EventKind::Review,
            _ => EventKind::Manual,
        };

        let max = max_cluster_receipts_for_kind(&kind);
        // At max-1, should be included
        if max > 0 && max < usize::MAX {
            prop_assert!(should_include_cluster_receipt(&kind, max - 1));
            prop_assert!(!should_include_cluster_receipt(&kind, max));
        }
    }

    #[test]
    fn prop_render_monotonicity(index in 0usize..100) {
        if !should_render_receipt_at(index) {
            prop_assert!(!should_render_receipt_at(index + 1));
        }
    }
}
