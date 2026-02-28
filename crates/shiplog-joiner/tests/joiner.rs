use proptest::prelude::*;
use shiplog_joiner::{InnerJoin, LeftJoin, StreamJoiner};
use std::collections::HashSet;

// ── InnerJoin correctness ──────────────────────────────────────────

#[test]
fn inner_join_basic() {
    let r = InnerJoin::new()
        .with_left_data(vec![(1, "a"), (2, "b"), (3, "c")])
        .with_right_data(vec![(2, "x"), (3, "y"), (4, "z")])
        .execute();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].left, "b");
    assert_eq!(r[0].right, "x");
    assert_eq!(r[1].left, "c");
    assert_eq!(r[1].right, "y");
}

#[test]
fn inner_join_no_matches() {
    let r = InnerJoin::new()
        .with_left_data(vec![(1, "a")])
        .with_right_data(vec![(2, "b")])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn inner_join_empty_left() {
    let r = InnerJoin::<&str, &str, i32>::new()
        .with_left_data(vec![])
        .with_right_data(vec![(1, "a")])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn inner_join_empty_right() {
    let r = InnerJoin::<&str, &str, i32>::new()
        .with_left_data(vec![(1, "a")])
        .with_right_data(vec![])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn inner_join_both_empty() {
    let r = InnerJoin::<&str, &str, i32>::new()
        .with_left_data(vec![])
        .with_right_data(vec![])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn inner_join_all_keys_match() {
    let r = InnerJoin::new()
        .with_left_data(vec![(1, "a"), (2, "b")])
        .with_right_data(vec![(1, "x"), (2, "y")])
        .execute();
    assert_eq!(r.len(), 2);
}

#[test]
fn inner_join_single_match() {
    let r = InnerJoin::new()
        .with_left_data(vec![(1, 10)])
        .with_right_data(vec![(1, 20)])
        .execute();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].left, 10);
    assert_eq!(r[0].right, 20);
}

// ── LeftJoin correctness ───────────────────────────────────────────

#[test]
fn left_join_basic() {
    let r = LeftJoin::new()
        .with_left_data(vec![(1, "a"), (2, "b"), (3, "c")])
        .with_right_data(vec![(1, "x"), (3, "z")])
        .execute();
    assert_eq!(r.len(), 3);
    assert_eq!(r[0].right, Some("x"));
    assert_eq!(r[1].right, None);
    assert_eq!(r[2].right, Some("z"));
}

#[test]
fn left_join_all_unmatched() {
    let r = LeftJoin::new()
        .with_left_data(vec![(1, "a"), (2, "b")])
        .with_right_data(vec![(10, "x")])
        .execute();
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|jr| jr.right.is_none()));
}

#[test]
fn left_join_empty_left() {
    let r = LeftJoin::<&str, &str, i32>::new()
        .with_left_data(vec![])
        .with_right_data(vec![(1i32, "a")])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn left_join_empty_right() {
    let r = LeftJoin::<&str, &str, i32>::new()
        .with_left_data(vec![(1, "a"), (2, "b")])
        .with_right_data(vec![])
        .execute();
    assert_eq!(r.len(), 2);
    assert!(r.iter().all(|jr| jr.right.is_none()));
}

#[test]
fn left_join_preserves_left_order() {
    let r = LeftJoin::new()
        .with_left_data(vec![(3, "c"), (1, "a"), (2, "b")])
        .with_right_data(vec![(1, "x"), (2, "y"), (3, "z")])
        .execute();
    assert_eq!(r[0].left, "c");
    assert_eq!(r[1].left, "a");
    assert_eq!(r[2].left, "b");
}

// ── StreamJoiner ───────────────────────────────────────────────────

#[test]
fn stream_joiner_group_by() {
    let joiner = StreamJoiner::new(|x: &i32| x % 3).add_many(vec![0, 1, 2, 3, 4, 5, 6]);
    let groups = joiner.group_by_key();
    assert_eq!(groups[&0], vec![0, 3, 6]);
    assert_eq!(groups[&1], vec![1, 4]);
    assert_eq!(groups[&2], vec![2, 5]);
}

#[test]
fn stream_joiner_empty() {
    let joiner = StreamJoiner::<i32, i32>::new(|x| *x);
    assert!(joiner.group_by_key().is_empty());
}

#[test]
fn stream_joiner_push_single() {
    let joiner = StreamJoiner::new(|x: &i32| *x).push(42);
    let groups = joiner.group_by_key();
    assert_eq!(groups[&42], vec![42]);
}

#[test]
fn stream_joiner_default() {
    let _: InnerJoin<i32, i32, i32> = InnerJoin::default();
    let _: LeftJoin<i32, i32, i32> = LeftJoin::default();
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_inner_join_result_bounded_by_left_size(
        left in prop::collection::vec((0u32..20, 0i32..100), 0..30),
        right in prop::collection::vec((0u32..20, 0i32..100), 0..30),
    ) {
        let r = InnerJoin::new()
            .with_left_data(left.clone())
            .with_right_data(right)
            .execute();
        // Result size <= left size (each left entry matches at most once)
        prop_assert!(r.len() <= left.len());
    }

    #[test]
    fn prop_left_join_preserves_left_count(
        left in prop::collection::vec((0u32..20, 0i32..100), 0..30),
        right in prop::collection::vec((0u32..20, 0i32..100), 0..30),
    ) {
        let r = LeftJoin::new()
            .with_left_data(left.clone())
            .with_right_data(right)
            .execute();
        prop_assert_eq!(r.len(), left.len());
    }

    #[test]
    fn prop_inner_join_commutative_count(
        left in prop::collection::vec((0u32..10, 0i32..100), 0..20),
        right in prop::collection::vec((0u32..10, 0i32..100), 0..20),
    ) {
        // Inner join on unique keys should have same count both ways
        let mut left_dedup: Vec<(u32, i32)> = Vec::new();
        let mut seen = HashSet::new();
        for (k, v) in &left {
            if seen.insert(*k) { left_dedup.push((*k, *v)); }
        }
        let mut right_dedup: Vec<(u32, i32)> = Vec::new();
        seen.clear();
        for (k, v) in &right {
            if seen.insert(*k) { right_dedup.push((*k, *v)); }
        }

        let r1 = InnerJoin::new()
            .with_left_data(left_dedup.clone())
            .with_right_data(right_dedup.clone())
            .execute();
        let r2 = InnerJoin::new()
            .with_left_data(right_dedup)
            .with_right_data(left_dedup)
            .execute();
        prop_assert_eq!(r1.len(), r2.len());
    }

    #[test]
    fn prop_stream_joiner_groups_cover_all(
        items in prop::collection::vec(0i32..50, 0..100)
    ) {
        let joiner = StreamJoiner::new(|x: &i32| x % 5).add_many(items.clone());
        let groups = joiner.group_by_key();
        let total: usize = groups.values().map(|v| v.len()).sum();
        prop_assert_eq!(total, items.len());
    }
}
