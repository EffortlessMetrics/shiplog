//! Snapshot tests for receipt policy behavior across all event kinds.

use shiplog_schema::event::EventKind;
use shiplog_workstream_receipt_policy::{
    max_cluster_receipts_for_kind, should_include_cluster_receipt, should_render_receipt_at,
};

#[test]
fn snapshot_receipt_limits_per_kind() {
    let limits: Vec<(&str, String)> = vec![
        (
            "PullRequest",
            format!("{}", max_cluster_receipts_for_kind(&EventKind::PullRequest)),
        ),
        (
            "Review",
            format!("{}", max_cluster_receipts_for_kind(&EventKind::Review)),
        ),
        (
            "Manual",
            format!("{}", max_cluster_receipts_for_kind(&EventKind::Manual)),
        ),
    ];

    let output = limits
        .iter()
        .map(|(kind, limit)| format!("{kind}: {limit}"))
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("receipt_limits_per_kind", output);
}

#[test]
fn snapshot_inclusion_table_for_reviews() {
    let output: String = (0..=10)
        .map(|count| {
            let included = should_include_cluster_receipt(&EventKind::Review, count);
            format!("count={count:>2} -> included={included}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("review_inclusion_table", output);
}

#[test]
fn snapshot_inclusion_table_for_manual() {
    let output: String = (0..=10)
        .map(|count| {
            let included = should_include_cluster_receipt(&EventKind::Manual, count);
            format!("count={count:>2} -> included={included}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("manual_inclusion_table", output);
}

#[test]
fn snapshot_render_visibility_table() {
    let output: String = (0..=10)
        .map(|index| {
            let visible = should_render_receipt_at(index);
            format!("index={index:>2} -> visible={visible}")
        })
        .collect::<Vec<_>>()
        .join("\n");

    insta::assert_snapshot!("render_visibility_table", output);
}
