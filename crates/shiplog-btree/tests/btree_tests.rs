use shiplog_btree::{BTree, BTreeNode};

#[test]
fn new_tree_is_empty() {
    let tree: BTree<i32> = BTree::new(3);
    assert!(tree.is_empty());
    assert_eq!(tree.size(), 0);
    assert_eq!(tree.degree(), 3);
}

#[test]
fn insert_increments_size() {
    let mut tree: BTree<i32> = BTree::new(3);
    tree.insert(10);
    tree.insert(20);
    tree.insert(30);
    assert_eq!(tree.size(), 3);
    assert!(!tree.is_empty());
}

#[test]
fn default_tree() {
    let tree: BTree<i32> = BTree::default();
    assert_eq!(tree.degree(), 3);
    assert!(tree.is_empty());
}

#[test]
fn btree_node_creation() {
    let node: BTreeNode<i32> = BTreeNode::new(true);
    assert!(node.is_leaf);
    assert!(node.values.is_empty());
    assert!(node.children.is_empty());
}

#[test]
fn search_returns_false() {
    let tree: BTree<i32> = BTree::new(3);
    assert!(!tree.search(&5));
}

#[test]
fn different_degrees() {
    for degree in [2, 3, 5, 10, 100] {
        let tree: BTree<i32> = BTree::new(degree);
        assert_eq!(tree.degree(), degree);
    }
}
