use proptest::prelude::*;
use shiplog_bst::{Bst, BstNode};

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn size_matches_insert_count(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut bst = Bst::new();
        for v in &values {
            bst.insert(*v);
        }
        prop_assert_eq!(bst.size(), values.len());
    }

    #[test]
    fn not_empty_after_insert(value: i32) {
        let mut bst = Bst::new();
        bst.insert(value);
        prop_assert!(!bst.is_empty());
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn new_bst_is_empty() {
    let bst: Bst<i32> = Bst::new();
    assert!(bst.is_empty());
    assert_eq!(bst.size(), 0);
}

#[test]
fn default_bst_is_empty() {
    let bst: Bst<i32> = Bst::default();
    assert!(bst.is_empty());
}

#[test]
fn single_insert() {
    let mut bst = Bst::new();
    bst.insert(42);
    assert_eq!(bst.size(), 1);
    assert!(!bst.is_empty());
}

#[test]
fn multiple_inserts() {
    let mut bst = Bst::new();
    for i in 0..50 {
        bst.insert(i);
    }
    assert_eq!(bst.size(), 50);
}

#[test]
fn search_returns_false_on_empty() {
    let bst: Bst<i32> = Bst::new();
    assert!(!bst.search(&42));
}

#[test]
fn bst_node_creation() {
    let node = BstNode::new(10);
    assert_eq!(node.value, 10);
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn bst_node_clone() {
    let node = BstNode::new(10);
    let cloned = node.clone();
    assert_eq!(cloned.value, 10);
}

// ── Stress test ─────────────────────────────────────────────────────────────

#[test]
fn stress_many_inserts() {
    let mut bst = Bst::new();
    for i in 0..10_000 {
        bst.insert(i);
    }
    assert_eq!(bst.size(), 10_000);
}
