use proptest::prelude::*;
use shiplog_tree::{AvlTree, BinaryNode, BinaryTree, Color, RbNode, RedBlackTree};

// ── Known-answer tests ──────────────────────────────────────────────

#[test]
fn binary_tree_size_tracks_inserts() {
    let mut t: BinaryTree<i32> = BinaryTree::new();
    assert!(t.is_empty());
    t.insert(10);
    t.insert(5);
    t.insert(15);
    assert_eq!(t.size(), 3);
    assert!(!t.is_empty());
}

#[test]
fn avl_tree_size_tracks_inserts() {
    let mut t: AvlTree<i32> = AvlTree::new();
    assert!(t.is_empty());
    t.insert(10);
    t.insert(5);
    assert_eq!(t.size(), 2);
}

#[test]
fn red_black_tree_size_tracks_inserts() {
    let mut t: RedBlackTree<i32> = RedBlackTree::new();
    assert!(t.is_empty());
    t.insert(10);
    assert_eq!(t.size(), 1);
}

#[test]
fn binary_node_construction() {
    let node = BinaryNode::new(42);
    assert_eq!(node.value, 42);
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn binary_node_with_children() {
    let mut node = BinaryNode::new(10);
    node.left = Some(Box::new(BinaryNode::new(5)));
    node.right = Some(Box::new(BinaryNode::new(15)));
    assert_eq!(node.left.as_ref().unwrap().value, 5);
    assert_eq!(node.right.as_ref().unwrap().value, 15);
}

#[test]
fn avl_node_initial_height() {
    let node = shiplog_tree::AvlNode::new(1);
    assert_eq!(node.height, 1);
    assert!(node.left.is_none());
    assert!(node.right.is_none());
}

#[test]
fn rb_node_color() {
    let node = RbNode {
        value: 1,
        left: None,
        right: None,
        color: Color::Red,
    };
    assert_eq!(node.color, Color::Red);

    let black_node = RbNode {
        value: 2,
        left: None,
        right: None,
        color: Color::Black,
    };
    assert_eq!(black_node.color, Color::Black);
}

#[test]
fn color_equality() {
    assert_eq!(Color::Red, Color::Red);
    assert_eq!(Color::Black, Color::Black);
    assert_ne!(Color::Red, Color::Black);
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn empty_trees_are_empty() {
    let bt: BinaryTree<i32> = BinaryTree::new();
    let at: AvlTree<i32> = AvlTree::new();
    let rbt: RedBlackTree<i32> = RedBlackTree::new();
    assert!(bt.is_empty());
    assert!(at.is_empty());
    assert!(rbt.is_empty());
    assert_eq!(bt.size(), 0);
    assert_eq!(at.size(), 0);
    assert_eq!(rbt.size(), 0);
}

#[test]
fn search_on_empty_tree_returns_false() {
    let bt: BinaryTree<i32> = BinaryTree::new();
    let at: AvlTree<i32> = AvlTree::new();
    let rbt: RedBlackTree<i32> = RedBlackTree::new();
    assert!(!bt.search(&42));
    assert!(!at.search(&42));
    assert!(!rbt.search(&42));
}

#[test]
fn single_insert_then_size_is_one() {
    let mut bt: BinaryTree<String> = BinaryTree::new();
    bt.insert("hello".to_string());
    assert_eq!(bt.size(), 1);
}

#[test]
fn binary_node_clone() {
    let node = BinaryNode::new(42);
    let cloned = node.clone();
    assert_eq!(cloned.value, 42);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn binary_tree_size_equals_insert_count(
        vals in prop::collection::vec(-1000i32..1000, 0..50)
    ) {
        let mut t: BinaryTree<i32> = BinaryTree::new();
        for v in &vals {
            t.insert(*v);
        }
        prop_assert_eq!(t.size(), vals.len());
    }

    #[test]
    fn avl_tree_size_equals_insert_count(
        vals in prop::collection::vec(-1000i32..1000, 0..50)
    ) {
        let mut t: AvlTree<i32> = AvlTree::new();
        for v in &vals {
            t.insert(*v);
        }
        prop_assert_eq!(t.size(), vals.len());
    }

    #[test]
    fn red_black_tree_size_equals_insert_count(
        vals in prop::collection::vec(-1000i32..1000, 0..50)
    ) {
        let mut t: RedBlackTree<i32> = RedBlackTree::new();
        for v in &vals {
            t.insert(*v);
        }
        prop_assert_eq!(t.size(), vals.len());
    }

    #[test]
    fn empty_after_zero_inserts(seed in 0u32..1000) {
        let _ = seed;
        let bt: BinaryTree<i32> = BinaryTree::new();
        let at: AvlTree<i32> = AvlTree::new();
        let rbt: RedBlackTree<i32> = RedBlackTree::new();
        prop_assert!(bt.is_empty());
        prop_assert!(at.is_empty());
        prop_assert!(rbt.is_empty());
    }

    #[test]
    fn not_empty_after_insert(val in -1000i32..1000) {
        let mut bt: BinaryTree<i32> = BinaryTree::new();
        bt.insert(val);
        prop_assert!(!bt.is_empty());
    }
}
