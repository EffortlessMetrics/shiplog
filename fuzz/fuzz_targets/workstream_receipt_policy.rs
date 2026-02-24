#![no_main]

use libfuzzer_sys::fuzz_target;
use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    should_include_cluster_receipt, should_render_receipt_at,
    truncate_cluster_receipts, WORKSTREAM_RECEIPT_LIMIT_REVIEW, WORKSTREAM_RECEIPT_LIMIT_MANUAL,
    WORKSTREAM_RECEIPT_LIMIT_TOTAL, WORKSTREAM_RECEIPT_RENDER_LIMIT,
};

fuzz_target!(|data: &[u8]| {
    let review_limit = WORKSTREAM_RECEIPT_LIMIT_REVIEW;
    let manual_limit = WORKSTREAM_RECEIPT_LIMIT_MANUAL;

    let idx = data.first().copied().unwrap_or(0) as usize;

    let kind = match data.get(1).copied().unwrap_or(0) % 3 {
        0 => EventKind::PullRequest,
        1 => EventKind::Review,
        _ => EventKind::Manual,
    };

    let count = data.get(2).copied().unwrap_or(0) as usize;
    let current_receipts = count % 64;
    let include_expected = match kind {
        EventKind::Review => current_receipts < review_limit,
        EventKind::Manual => current_receipts < manual_limit,
        EventKind::PullRequest => true,
    };
    assert_eq!(
        should_include_cluster_receipt(&kind, current_receipts),
        include_expected
    );

    assert_eq!(should_render_receipt_at(idx % 64), (idx % 64) < WORKSTREAM_RECEIPT_RENDER_LIMIT);

    let cap_len = (data.get(3).copied().unwrap_or(0) as usize) % 64;
    let mut receipts = vec![0usize; cap_len];
    truncate_cluster_receipts(&mut receipts);
    assert!(receipts.len() <= WORKSTREAM_RECEIPT_LIMIT_TOTAL);

    if cap_len > WORKSTREAM_RECEIPT_LIMIT_TOTAL {
        assert_eq!(receipts.len(), WORKSTREAM_RECEIPT_LIMIT_TOTAL);
    }
});
