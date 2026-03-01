use proptest::prelude::*;
use shiplog_cogrouper::{CoGrouper, StreamCoGrouper};
use std::collections::HashSet;

// ── CoGrouper correctness ──────────────────────────────────────────

#[test]
fn cogrouper_two_streams() {
    let r = CoGrouper::new()
        .add_stream(vec![(1, "a"), (2, "b")])
        .add_stream(vec![(1, "x"), (3, "y")])
        .execute();

    assert_eq!(r.len(), 3);
    assert_eq!(r[&1].groups[0], vec!["a"]);
    assert_eq!(r[&1].groups[1], vec!["x"]);
    assert_eq!(r[&2].groups[0], vec!["b"]);
    assert!(r[&2].groups[1].is_empty());
    assert!(r[&3].groups[0].is_empty());
    assert_eq!(r[&3].groups[1], vec!["y"]);
}

#[test]
fn cogrouper_three_streams() {
    let r = CoGrouper::new()
        .add_stream(vec![(1, "a")])
        .add_stream(vec![(1, "b")])
        .add_stream(vec![(1, "c")])
        .execute();

    assert_eq!(r.len(), 1);
    assert_eq!(r[&1].groups.len(), 3);
    assert_eq!(r[&1].groups[0], vec!["a"]);
    assert_eq!(r[&1].groups[1], vec!["b"]);
    assert_eq!(r[&1].groups[2], vec!["c"]);
}

#[test]
fn cogrouper_empty() {
    let r = CoGrouper::<i32, &str>::new().execute();
    assert!(r.is_empty());
}

#[test]
fn cogrouper_single_stream() {
    let r = CoGrouper::new()
        .add_stream(vec![(1, "a"), (2, "b"), (1, "c")])
        .execute();
    assert_eq!(r.len(), 2);
    assert_eq!(r[&1].groups[0], vec!["a", "c"]);
    assert_eq!(r[&2].groups[0], vec!["b"]);
}

#[test]
fn cogrouper_empty_stream_with_nonempty() {
    let r = CoGrouper::new()
        .add_stream(vec![])
        .add_stream(vec![(1, "a")])
        .execute();
    assert_eq!(r.len(), 1);
    assert!(r[&1].groups[0].is_empty());
    assert_eq!(r[&1].groups[1], vec!["a"]);
}

#[test]
fn cogrouper_disjoint_keys() {
    let r = CoGrouper::new()
        .add_stream(vec![(1, "a")])
        .add_stream(vec![(2, "b")])
        .execute();
    assert_eq!(r.len(), 2);
    assert_eq!(r[&1].groups[0], vec!["a"]);
    assert!(r[&1].groups[1].is_empty());
    assert!(r[&2].groups[0].is_empty());
    assert_eq!(r[&2].groups[1], vec!["b"]);
}

// ── common_keys / all_keys ─────────────────────────────────────────

#[test]
fn common_keys_intersection() {
    let mut keys = CoGrouper::new()
        .add_stream(vec![(1, "a"), (2, "b"), (3, "c")])
        .add_stream(vec![(2, "x"), (3, "y"), (4, "z")])
        .common_keys();
    keys.sort();
    assert_eq!(keys, vec![2, 3]);
}

#[test]
fn common_keys_empty() {
    let keys = CoGrouper::<i32, &str>::new().common_keys();
    assert!(keys.is_empty());
}

#[test]
fn common_keys_disjoint() {
    let keys = CoGrouper::new()
        .add_stream(vec![(1, "a")])
        .add_stream(vec![(2, "b")])
        .common_keys();
    assert!(keys.is_empty());
}

#[test]
fn all_keys_union() {
    let keys = CoGrouper::new()
        .add_stream(vec![(1, "a"), (2, "b")])
        .add_stream(vec![(2, "x"), (3, "y")])
        .all_keys();
    assert_eq!(keys.len(), 3);
    let set: HashSet<_> = keys.into_iter().collect();
    assert!(set.contains(&1));
    assert!(set.contains(&2));
    assert!(set.contains(&3));
}

#[test]
fn all_keys_empty() {
    let keys = CoGrouper::<i32, &str>::new().all_keys();
    assert!(keys.is_empty());
}

// ── StreamCoGrouper ────────────────────────────────────────────────

#[test]
fn stream_cogrouper_basic() {
    let r = StreamCoGrouper::new(|x: &i32| x % 2)
        .add_stream(vec![1, 2, 3])
        .add_stream(vec![4, 5, 6])
        .execute();
    assert_eq!(r.len(), 2);
    // key 0: stream0=[2], stream1=[4,6]
    assert_eq!(r[&0][0], vec![2]);
    assert_eq!(r[&0][1], vec![4, 6]);
    // key 1: stream0=[1,3], stream1=[5]
    assert_eq!(r[&1][0], vec![1, 3]);
    assert_eq!(r[&1][1], vec![5]);
}

#[test]
fn stream_cogrouper_empty() {
    let r = StreamCoGrouper::<i32, i32>::new(|x| *x).execute();
    assert!(r.is_empty());
}

#[test]
fn stream_cogrouper_single_item() {
    let r = StreamCoGrouper::new(|x: &i32| *x)
        .add_stream(vec![42])
        .execute();
    assert_eq!(r.len(), 1);
    assert_eq!(r[&42][0], vec![42]);
}

// ── Composition: CoGrouper -> filter common ────────────────────────

#[test]
fn composition_cogroup_then_filter_common() {
    let cg = CoGrouper::new()
        .add_stream(vec![(1, "a"), (2, "b"), (3, "c")])
        .add_stream(vec![(2, "x"), (3, "y"), (4, "z")]);

    let common = cg.common_keys();
    let result = cg.execute();

    let common_set: HashSet<_> = common.into_iter().collect();
    let filtered: Vec<_> = result
        .into_iter()
        .filter(|(k, _)| common_set.contains(k))
        .collect();

    assert_eq!(filtered.len(), 2);
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_cogroup_all_keys_superset_of_common(
        s1 in prop::collection::vec((0u32..10, 0i32..100), 0..20),
        s2 in prop::collection::vec((0u32..10, 0i32..100), 0..20),
    ) {
        let cg = CoGrouper::new()
            .add_stream(s1)
            .add_stream(s2);
        let common: HashSet<_> = cg.common_keys().into_iter().collect();
        let all: HashSet<_> = cg.all_keys().into_iter().collect();
        for k in &common {
            prop_assert!(all.contains(k));
        }
    }

    #[test]
    fn prop_cogroup_preserves_total_items(
        s1 in prop::collection::vec((0u32..10, 0i32..100), 0..20),
        s2 in prop::collection::vec((0u32..10, 0i32..100), 0..20),
    ) {
        let total_in = s1.len() + s2.len();
        let result = CoGrouper::new()
            .add_stream(s1)
            .add_stream(s2)
            .execute();
        let total_out: usize = result.values()
            .flat_map(|cg| cg.groups.iter())
            .map(|g| g.len())
            .sum();
        prop_assert_eq!(total_out, total_in);
    }

    #[test]
    fn prop_cogroup_groups_count_matches_streams(
        s1 in prop::collection::vec((0u32..10, 0i32..100), 1..10),
        s2 in prop::collection::vec((0u32..10, 0i32..100), 1..10),
    ) {
        let result = CoGrouper::new()
            .add_stream(s1)
            .add_stream(s2)
            .execute();
        for cg in result.values() {
            prop_assert_eq!(cg.groups.len(), 2);
        }
    }
}
