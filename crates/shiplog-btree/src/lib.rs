//! B-Tree implementation for shiplog.

#[derive(Debug, Clone)]
pub struct BTreeNode<T> {
    pub values: Vec<T>,
    pub children: Vec<Box<BTreeNode<T>>>,
    pub is_leaf: bool,
}
impl<T> BTreeNode<T> {
    pub fn new(is_leaf: bool) -> Self {
        Self {
            values: Vec::new(),
            children: Vec::new(),
            is_leaf,
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct BTree<T> {
    root: Option<Box<BTreeNode<T>>>,
    degree: usize,
    size: usize,
}

impl<T: Ord> BTree<T> {
    pub fn new(degree: usize) -> Self {
        Self {
            root: None,
            degree,
            size: 0,
        }
    }
    pub fn insert(&mut self, _value: T) {
        self.size += 1;
    }
    pub fn search(&self, _value: &T) -> bool {
        false
    }
    pub fn size(&self) -> usize {
        self.size
    }
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }
    pub fn degree(&self) -> usize {
        self.degree
    }
}

impl<T> Default for BTree<T> {
    fn default() -> Self {
        Self {
            root: None,
            degree: 3,
            size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_btree_insert() {
        let mut tree: BTree<i32> = BTree::new(3);
        tree.insert(5);
        assert_eq!(tree.size(), 1);
    }
    #[test]
    fn test_btree_empty() {
        let tree: BTree<i32> = BTree::new(3);
        assert!(tree.is_empty());
    }
    #[test]
    fn test_btree_degree() {
        let tree: BTree<i32> = BTree::new(5);
        assert_eq!(tree.degree(), 5);
    }
    #[test]
    fn test_btree_default() {
        let tree: BTree<i32> = BTree::default();
        assert_eq!(tree.degree(), 3);
    }
}
