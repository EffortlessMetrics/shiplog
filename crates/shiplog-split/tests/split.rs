use proptest::prelude::*;
use shiplog_split::{RoundRobinSplitter, StreamSplitter};

// ── split_by predicate ─────────────────────────────────────────────

#[test]
fn split_by_even_odd() {
    let r = StreamSplitter::new()
        .with_data(vec![1, 2, 3, 4, 5])
        .split_by(|x| x % 2 == 0);
    assert_eq!(r.matched, vec![2, 4]);
    assert_eq!(r.unmatched, vec![1, 3, 5]);
}

#[test]
fn split_by_all_match() {
    let r = StreamSplitter::new()
        .with_data(vec![2, 4, 6])
        .split_by(|x| x % 2 == 0);
    assert_eq!(r.matched, vec![2, 4, 6]);
    assert!(r.unmatched.is_empty());
}

#[test]
fn split_by_none_match() {
    let r = StreamSplitter::new()
        .with_data(vec![1, 3, 5])
        .split_by(|x| x % 2 == 0);
    assert!(r.matched.is_empty());
    assert_eq!(r.unmatched, vec![1, 3, 5]);
}

#[test]
fn split_by_empty() {
    let r = StreamSplitter::<i32>::new()
        .with_data(vec![])
        .split_by(|_| true);
    assert!(r.matched.is_empty());
    assert!(r.unmatched.is_empty());
}

// ── split_into_n ───────────────────────────────────────────────────

#[test]
fn split_into_n_even() {
    let r = StreamSplitter::new()
        .with_data(vec![1, 2, 3, 4, 5, 6])
        .split_into_n(3);
    assert_eq!(r.len(), 3);
    assert_eq!(r[0], vec![1, 2]);
    assert_eq!(r[1], vec![3, 4]);
    assert_eq!(r[2], vec![5, 6]);
}

#[test]
fn split_into_n_zero() {
    let r = StreamSplitter::new()
        .with_data(vec![1, 2, 3])
        .split_into_n(0);
    assert!(r.is_empty());
}

#[test]
fn split_into_n_more_than_items() {
    let r = StreamSplitter::new().with_data(vec![1, 2]).split_into_n(5);
    assert_eq!(r.len(), 5);
    let total: usize = r.iter().map(|v| v.len()).sum();
    assert_eq!(total, 2);
}

#[test]
fn split_into_n_single() {
    let r = StreamSplitter::new()
        .with_data(vec![1, 2, 3])
        .split_into_n(1);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], vec![1, 2, 3]);
}

#[test]
fn split_into_n_empty_data() {
    let r = StreamSplitter::<i32>::new()
        .with_data(vec![])
        .split_into_n(3);
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|v| v.is_empty()));
}

// ── take / skip / split_at ─────────────────────────────────────────

#[test]
fn take_fewer_than_available() {
    let r = StreamSplitter::new().with_data(vec![1, 2, 3, 4, 5]).take(3);
    assert_eq!(r, vec![1, 2, 3]);
}

#[test]
fn take_more_than_available() {
    let r = StreamSplitter::new().with_data(vec![1, 2]).take(10);
    assert_eq!(r, vec![1, 2]);
}

#[test]
fn take_zero() {
    let r = StreamSplitter::new().with_data(vec![1, 2, 3]).take(0);
    assert!(r.is_empty());
}

#[test]
fn skip_fewer_than_available() {
    let r = StreamSplitter::new().with_data(vec![1, 2, 3, 4, 5]).skip(2);
    assert_eq!(r, vec![3, 4, 5]);
}

#[test]
fn skip_all() {
    let r = StreamSplitter::new().with_data(vec![1, 2]).skip(5);
    assert!(r.is_empty());
}

#[test]
fn split_at_middle() {
    let (a, b) = StreamSplitter::new()
        .with_data(vec![1, 2, 3, 4, 5])
        .split_at(3);
    assert_eq!(a, vec![1, 2, 3]);
    assert_eq!(b, vec![4, 5]);
}

#[test]
fn split_at_zero() {
    let (a, b) = StreamSplitter::new().with_data(vec![1, 2, 3]).split_at(0);
    assert!(a.is_empty());
    assert_eq!(b, vec![1, 2, 3]);
}

#[test]
fn split_at_beyond_end() {
    let (a, b) = StreamSplitter::new().with_data(vec![1, 2, 3]).split_at(10);
    assert_eq!(a, vec![1, 2, 3]);
    assert!(b.is_empty());
}

// ── group_by_key ───────────────────────────────────────────────────

#[test]
fn group_by_key_basic() {
    let groups = StreamSplitter::new()
        .with_data(vec![1, 2, 3, 4, 5, 6])
        .group_by_key(|x| x % 2);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[&0], vec![2, 4, 6]);
    assert_eq!(groups[&1], vec![1, 3, 5]);
}

// ── partition_by ───────────────────────────────────────────────────

#[test]
fn partition_by_modulo() {
    let parts = StreamSplitter::new()
        .with_data(vec![0, 1, 2, 3, 4, 5])
        .partition_by(|x| x % 3);
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], vec![0, 3]);
    assert_eq!(parts[1], vec![1, 4]);
    assert_eq!(parts[2], vec![2, 5]);
}

// ── RoundRobinSplitter ────────────────────────────────────────────

#[test]
fn round_robin_basic() {
    let r = RoundRobinSplitter::new(3)
        .with_data(vec![1, 2, 3, 4, 5, 6])
        .execute();
    assert_eq!(r[0], vec![1, 4]);
    assert_eq!(r[1], vec![2, 5]);
    assert_eq!(r[2], vec![3, 6]);
}

#[test]
fn round_robin_zero_streams() {
    let r = RoundRobinSplitter::new(0)
        .with_data(vec![1, 2, 3])
        .execute();
    assert!(r.is_empty());
}

#[test]
fn round_robin_empty_data() {
    let r = RoundRobinSplitter::<i32>::new(3)
        .with_data(vec![])
        .execute();
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|v| v.is_empty()));
}

#[test]
fn round_robin_single_stream() {
    let r = RoundRobinSplitter::new(1)
        .with_data(vec![1, 2, 3])
        .execute();
    assert_eq!(r.len(), 1);
    assert_eq!(r[0], vec![1, 2, 3]);
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_split_by_preserves_all_elements(data in prop::collection::vec(0i32..100, 0..100)) {
        let r = StreamSplitter::new()
            .with_data(data.clone())
            .split_by(|x| x % 2 == 0);
        prop_assert_eq!(r.matched.len() + r.unmatched.len(), data.len());
    }

    #[test]
    fn prop_split_into_n_preserves_total(
        data in prop::collection::vec(0i32..100, 0..100),
        n in 1usize..10
    ) {
        let r = StreamSplitter::new()
            .with_data(data.clone())
            .split_into_n(n);
        let total: usize = r.iter().map(|v| v.len()).sum();
        prop_assert_eq!(total, data.len());
    }

    #[test]
    fn prop_take_skip_reconstruct(
        data in prop::collection::vec(0i32..100, 0..100),
        k in 0usize..100
    ) {
        let taken = StreamSplitter::new().with_data(data.clone()).take(k);
        let skipped = StreamSplitter::new().with_data(data.clone()).skip(k);
        let mut reconstructed = taken;
        reconstructed.extend(skipped);
        prop_assert_eq!(reconstructed, data);
    }

    #[test]
    fn prop_split_at_reconstructs(
        data in prop::collection::vec(0i32..100, 0..100),
        k in 0usize..100
    ) {
        let (a, b) = StreamSplitter::new().with_data(data.clone()).split_at(k);
        let mut reconstructed = a;
        reconstructed.extend(b);
        prop_assert_eq!(reconstructed, data);
    }

    #[test]
    fn prop_round_robin_preserves_total(
        data in prop::collection::vec(0i32..100, 0..100),
        n in 1usize..10
    ) {
        let r = RoundRobinSplitter::new(n).with_data(data.clone()).execute();
        let total: usize = r.iter().map(|v| v.len()).sum();
        prop_assert_eq!(total, data.len());
    }

    #[test]
    fn prop_round_robin_balanced(
        data in prop::collection::vec(0i32..100, 0..100),
        n in 1usize..10
    ) {
        let r = RoundRobinSplitter::new(n).with_data(data.clone()).execute();
        // Max difference between any two partitions should be at most 1
        if !r.is_empty() {
            let max_len = r.iter().map(|v| v.len()).max().unwrap_or(0);
            let min_len = r.iter().map(|v| v.len()).min().unwrap_or(0);
            prop_assert!(max_len - min_len <= 1);
        }
    }
}
