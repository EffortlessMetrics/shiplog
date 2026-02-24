// Property tests for workstream receipt policy functions.

use proptest::prelude::*;
use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    should_include_cluster_receipt, should_render_receipt_at, truncate_cluster_receipts,
    WORKSTREAM_RECEIPT_LIMIT_MANUAL, WORKSTREAM_RECEIPT_LIMIT_REVIEW, WORKSTREAM_RECEIPT_LIMIT_TOTAL,
    WORKSTREAM_RECEIPT_RENDER_LIMIT,
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
}
