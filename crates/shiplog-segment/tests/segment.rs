use proptest::prelude::*;
use shiplog_segment::{BinarySearchTree, Interval, SegmentTree, TreeNode};

// ── Interval known-answer tests ─────────────────────────────────────

#[test]
fn interval_contains_start_excludes_end() {
    let i = Interval::new(5, 10);
    assert!(i.contains(5));
    assert!(i.contains(9));
    assert!(!i.contains(10));
    assert!(!i.contains(4));
}

#[test]
fn interval_overlap_partial() {
    let a = Interval::new(0, 10);
    let b = Interval::new(5, 15);
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn interval_overlap_adjacent_no_overlap() {
    let a = Interval::new(0, 10);
    let b = Interval::new(10, 20);
    assert!(!a.overlaps(&b));
    assert!(!b.overlaps(&a));
}

#[test]
fn interval_overlap_contained() {
    let a = Interval::new(0, 20);
    let b = Interval::new(5, 10);
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

#[test]
fn interval_len_and_empty() {
    assert_eq!(Interval::new(0, 10).len(), 10);
    assert_eq!(Interval::new(5, 5).len(), 0);
    assert!(Interval::new(5, 5).is_empty());
    assert!(Interval::new(10, 5).is_empty());
    assert!(!Interval::new(0, 1).is_empty());
}

#[test]
fn interval_ordering() {
    let mut intervals = [
        Interval::new(10, 20),
        Interval::new(0, 5),
        Interval::new(0, 10),
        Interval::new(5, 15),
    ];
    intervals.sort();
    assert_eq!(intervals[0], Interval::new(0, 5));
    assert_eq!(intervals[1], Interval::new(0, 10));
    assert_eq!(intervals[2], Interval::new(5, 15));
    assert_eq!(intervals[3], Interval::new(10, 20));
}

// ── SegmentTree tests ───────────────────────────────────────────────

#[test]
fn segment_tree_get_set() {
    let mut tree: SegmentTree<i32> = SegmentTree::new(4);
    tree.set(0, 10);
    tree.set(3, 40);
    assert_eq!(tree.get(0), Some(&10));
    assert_eq!(tree.get(3), Some(&40));
}

#[test]
fn segment_tree_get_out_of_bounds() {
    let tree: SegmentTree<i32> = SegmentTree::new(4);
    assert_eq!(tree.get(10), None);
}

#[test]
fn segment_tree_set_out_of_bounds_noop() {
    let mut tree: SegmentTree<i32> = SegmentTree::new(4);
    tree.set(100, 999);
    // No panic, get still returns None for out-of-bounds
    assert_eq!(tree.get(100), None);
}

#[test]
fn segment_tree_from_slice_preserves_values() {
    let data = vec![10, 20, 30, 40];
    let tree = SegmentTree::from_slice(&data);
    for (i, &v) in data.iter().enumerate() {
        assert_eq!(tree.get(i), Some(&v));
    }
}

#[test]
fn segment_tree_query_empty_range() {
    let tree = SegmentTree::from_slice(&[1, 2, 3, 4]);
    assert_eq!(tree.query(2, 2), 0); // empty range [2,2)
    assert_eq!(tree.query(5, 10), 0); // out of bounds
    assert_eq!(tree.query(3, 1), 0); // reversed
}

// ── BinarySearchTree tests ──────────────────────────────────────────

#[test]
fn bst_insert_and_search() {
    let mut bst = BinarySearchTree::new();
    bst.insert(5);
    bst.insert(3);
    bst.insert(7);
    bst.insert(1);
    bst.insert(9);

    assert!(bst.search(&5));
    assert!(bst.search(&3));
    assert!(bst.search(&7));
    assert!(bst.search(&1));
    assert!(bst.search(&9));
    assert!(!bst.search(&0));
    assert!(!bst.search(&6));
}

#[test]
fn bst_empty() {
    let bst: BinarySearchTree<i32> = BinarySearchTree::new();
    assert!(bst.is_empty());
    assert!(!bst.search(&1));
}

#[test]
fn bst_single_element() {
    let mut bst = BinarySearchTree::new();
    bst.insert(42);
    assert!(!bst.is_empty());
    assert!(bst.search(&42));
    assert!(!bst.search(&0));
}

#[test]
fn bst_duplicates() {
    let mut bst = BinarySearchTree::new();
    bst.insert(5);
    bst.insert(5);
    bst.insert(5);
    assert!(bst.search(&5));
}

#[test]
fn bst_sorted_insertion() {
    let mut bst = BinarySearchTree::new();
    for i in 0..10 {
        bst.insert(i);
    }
    for i in 0..10 {
        assert!(bst.search(&i));
    }
}

#[test]
fn bst_default() {
    let bst: BinarySearchTree<i32> = BinarySearchTree::default();
    assert!(bst.is_empty());
}

// ── TreeNode tests ──────────────────────────────────────────────────

#[test]
fn tree_node_builder_pattern() {
    let tree = TreeNode::new(1)
        .with_left(TreeNode::new(2).with_left(TreeNode::new(4)))
        .with_right(TreeNode::new(3));

    assert_eq!(tree.value, 1);
    assert_eq!(tree.left.as_ref().unwrap().value, 2);
    assert_eq!(tree.right.as_ref().unwrap().value, 3);
    assert_eq!(tree.left.as_ref().unwrap().left.as_ref().unwrap().value, 4);
}

#[test]
fn tree_node_leaf() {
    let node = TreeNode::new("leaf");
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn interval_overlap_symmetric(
        s1 in -100i64..100, len1 in 1i64..50,
        s2 in -100i64..100, len2 in 1i64..50
    ) {
        let a = Interval::new(s1, s1 + len1);
        let b = Interval::new(s2, s2 + len2);
        prop_assert_eq!(a.overlaps(&b), b.overlaps(&a));
    }

    #[test]
    fn interval_contains_all_interior_points(
        start in -100i64..100, len in 1i64..50
    ) {
        let i = Interval::new(start, start + len);
        for p in start..start + len {
            prop_assert!(i.contains(p));
        }
        prop_assert!(!i.contains(start + len)); // end excluded
    }

    #[test]
    fn interval_non_empty_has_positive_length(
        start in -100i64..100, len in 1i64..50
    ) {
        let i = Interval::new(start, start + len);
        prop_assert!(!i.is_empty());
        prop_assert!(!i.is_empty());
    }

    #[test]
    fn bst_all_inserted_found(
        vals in prop::collection::vec(-500i32..500, 1..50)
    ) {
        let mut bst = BinarySearchTree::new();
        for &v in &vals {
            bst.insert(v);
        }
        for &v in &vals {
            prop_assert!(bst.search(&v));
        }
    }

    #[test]
    fn bst_not_empty_after_insert(val in -1000i32..1000) {
        let mut bst = BinarySearchTree::new();
        bst.insert(val);
        prop_assert!(!bst.is_empty());
    }

    #[test]
    fn segment_tree_set_get_round_trip(
        size in 1usize..32, idx in 0usize..32, val in -1000i32..1000
    ) {
        let mut tree: SegmentTree<i32> = SegmentTree::new(size);
        if idx < size {
            tree.set(idx, val);
            prop_assert_eq!(tree.get(idx), Some(&val));
        }
    }
}
