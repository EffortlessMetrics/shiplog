use proptest::prelude::*;
use shiplog_fenwick::{FenwickTree, FenwickTree2D};

// ── Known-answer tests ──────────────────────────────────────────────

#[test]
fn prefix_sums_match_naive() {
    let vals = vec![3, 1, 4, 1, 5, 9, 2, 6];
    let tree = FenwickTree::from_slice(&vals);
    let mut prefix = 0i32;
    for (i, &v) in vals.iter().enumerate() {
        prefix += v;
        assert_eq!(tree.sum(i), prefix, "prefix sum mismatch at index {i}");
    }
}

#[test]
fn range_sum_known_values() {
    let tree = FenwickTree::from_slice(&[10, 20, 30, 40, 50]);
    assert_eq!(tree.range_sum(0, 4), 150);
    assert_eq!(tree.range_sum(1, 3), 90);
    assert_eq!(tree.range_sum(2, 2), 30);
}

#[test]
fn get_returns_individual_values() {
    let vals = vec![5, 10, 15, 20];
    let tree = FenwickTree::from_slice(&vals);
    for (i, &v) in vals.iter().enumerate() {
        assert_eq!(tree.get(i), v);
    }
}

#[test]
fn lower_bound_known() {
    // values: [1, 2, 3, 4, 5]  prefix sums: [1, 3, 6, 10, 15]
    let tree = FenwickTree::from_slice(&[1, 2, 3, 4, 5]);
    assert_eq!(tree.lower_bound(0), Some(0));
    assert_eq!(tree.lower_bound(1), Some(0));
    assert_eq!(tree.lower_bound(4), Some(2));
    assert_eq!(tree.lower_bound(15), Some(4));
    assert_eq!(tree.lower_bound(16), None);
}

#[test]
fn add_then_get_round_trip() {
    let mut tree: FenwickTree<i64> = FenwickTree::new(4);
    tree.add(0, 100);
    tree.add(1, 200);
    tree.add(2, 300);
    tree.add(3, 400);
    assert_eq!(tree.get(0), 100);
    assert_eq!(tree.get(1), 200);
    assert_eq!(tree.get(2), 300);
    assert_eq!(tree.get(3), 400);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn empty_tree() {
    let tree: FenwickTree<i32> = FenwickTree::new(0);
    assert!(tree.is_empty());
    assert_eq!(tree.len(), 0);
}

#[test]
fn single_element() {
    let tree = FenwickTree::from_slice(&[42]);
    assert_eq!(tree.len(), 1);
    assert_eq!(tree.sum(0), 42);
    assert_eq!(tree.get(0), 42);
    assert_eq!(tree.range_sum(0, 0), 42);
}

#[test]
fn get_out_of_bounds_returns_default() {
    let tree = FenwickTree::from_slice(&[1, 2, 3]);
    assert_eq!(tree.get(100), 0);
}

#[test]
fn range_sum_reversed_returns_zero() {
    let tree = FenwickTree::from_slice(&[1, 2, 3, 4]);
    assert_eq!(tree.range_sum(3, 1), 0);
}

#[test]
fn range_sum_out_of_bounds_returns_zero() {
    let tree = FenwickTree::from_slice(&[1, 2, 3]);
    assert_eq!(tree.range_sum(0, 10), 0);
}

#[test]
fn from_slice_empty() {
    let tree = FenwickTree::from_slice(&[] as &[i32]);
    assert!(tree.is_empty());
}

// ── 2D known-answer tests ──────────────────────────────────────────

#[test]
fn fenwick2d_single_cell() {
    let mut tree = FenwickTree2D::new(1, 1);
    tree.add(0, 0, 7);
    assert_eq!(tree.sum(0, 0), 7);
    assert_eq!(tree.range_sum(0, 0, 0, 0), 7);
}

#[test]
fn fenwick2d_3x3_matrix() {
    let mut tree = FenwickTree2D::new(3, 3);
    // Fill: [[1,2,3],[4,5,6],[7,8,9]]
    for r in 0..3 {
        for c in 0..3 {
            tree.add(r, c, (r * 3 + c + 1) as i32);
        }
    }
    // Full matrix sum
    assert_eq!(tree.range_sum(0, 0, 2, 2), 45);
    // Sub-matrix [1..2, 1..2] = 5+6+8+9 = 28
    assert_eq!(tree.range_sum(1, 1, 2, 2), 28);
    // Single row [0, 0..2] = 1+2+3 = 6
    assert_eq!(tree.range_sum(0, 0, 0, 2), 6);
}

#[test]
fn fenwick2d_invalid_range() {
    let tree: FenwickTree2D<i32> = FenwickTree2D::new(3, 3);
    assert_eq!(tree.range_sum(2, 0, 1, 0), 0); // r1 > r2
    assert_eq!(tree.range_sum(0, 2, 0, 1), 0); // c1 > c2
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn prefix_sum_matches_naive(vals in prop::collection::vec(0i64..1000, 1..64)) {
        let tree = FenwickTree::from_slice(&vals);
        let mut naive = 0i64;
        for (i, &v) in vals.iter().enumerate() {
            naive += v;
            prop_assert_eq!(tree.sum(i), naive);
        }
    }

    #[test]
    fn range_sum_equals_sum_difference(vals in prop::collection::vec(0i64..1000, 2..64)) {
        let tree = FenwickTree::from_slice(&vals);
        let n = vals.len();
        let l = 0usize;
        let r = n - 1;
        let expected: i64 = vals[l..=r].iter().sum();
        prop_assert_eq!(tree.range_sum(l, r), expected);
    }

    #[test]
    fn get_recovers_original_values(vals in prop::collection::vec(0i64..1000, 1..64)) {
        let tree = FenwickTree::from_slice(&vals);
        for (i, &v) in vals.iter().enumerate() {
            prop_assert_eq!(tree.get(i), v);
        }
    }

    #[test]
    fn add_is_cumulative(base in 0i64..500, delta in 0i64..500) {
        let mut tree = FenwickTree::new(1);
        tree.add(0, base);
        tree.add(0, delta);
        prop_assert_eq!(tree.get(0), base + delta);
    }

    #[test]
    fn len_equals_construction_size(n in 0usize..256) {
        let tree: FenwickTree<i32> = FenwickTree::new(n);
        prop_assert_eq!(tree.len(), n);
        prop_assert_eq!(tree.is_empty(), n == 0);
    }

    #[test]
    fn lower_bound_none_when_target_exceeds_total(vals in prop::collection::vec(1i64..100, 1..32)) {
        let tree = FenwickTree::from_slice(&vals);
        let total: i64 = vals.iter().sum();
        prop_assert_eq!(tree.lower_bound(total + 1), None);
    }

    #[test]
    fn fenwick2d_point_add_sum_round_trip(
        r in 0usize..8, c in 0usize..8, v in 1i64..1000
    ) {
        let mut tree = FenwickTree2D::new(8, 8);
        tree.add(r, c, v);
        prop_assert_eq!(tree.range_sum(r, c, r, c), v);
    }
}
