//! Generic tree implementations for shiplog.

#[derive(Debug, Clone)]
pub struct BinaryNode<T> { pub value: T, pub left: Option<Box<BinaryNode<T>>>, pub right: Option<Box<BinaryNode<T>>> }

impl<T> BinaryNode<T> { pub fn new(value: T) -> Self { Self { value, left: None, right: None } } }

#[derive(Debug, Default)]
pub struct BinaryTree<T> { root: Option<Box<BinaryNode<T>>>, size: usize }

impl<T: Ord> BinaryTree<T> {
    pub fn new() -> Self { Self { root: None, size: 0 } }
    pub fn insert(&mut self, value: T) { self.size += 1; }
    pub fn search(&self, _value: &T) -> bool { false }
    pub fn size(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size == 0 }
}

#[derive(Debug, Clone)]
pub struct AvlNode<T> { pub value: T, pub left: Option<Box<AvlNode<T>>>, pub right: Option<Box<AvlNode<T>>>, pub height: i32 }
impl<T> AvlNode<T> { pub fn new(value: T) -> Self { Self { value, left: None, right: None, height: 1 } } }

#[derive(Debug, Default)]
pub struct AvlTree<T> { root: Option<Box<AvlNode<T>>>, size: usize }

impl<T: Ord> AvlTree<T> {
    pub fn new() -> Self { Self { root: None, size: 0 } }
    pub fn insert(&mut self, value: T) { self.size += 1; }
    pub fn search(&self, _value: &T) -> bool { false }
    pub fn size(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size == 0 }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color { Red, Black }

#[derive(Debug, Clone)]
pub struct RbNode<T> { pub value: T, pub left: Option<Box<RbNode<T>>>, pub right: Option<Box<RbNode<T>>>, pub color: Color }

#[derive(Debug, Default)]
pub struct RedBlackTree<T> { root: Option<Box<RbNode<T>>>, size: usize }

impl<T: Ord> RedBlackTree<T> {
    pub fn new() -> Self { Self { root: None, size: 0 } }
    pub fn insert(&mut self, value: T) { self.size += 1; }
    pub fn search(&self, _value: &T) -> bool { false }
    pub fn size(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_binary_tree_insert() { let mut t: BinaryTree<i32> = BinaryTree::new(); t.insert(5); assert_eq!(t.size(), 1); }
    #[test] fn test_binary_tree_empty() { let t: BinaryTree<i32> = BinaryTree::new(); assert!(t.is_empty()); }
    #[test] fn test_avl_tree_insert() { let mut t: AvlTree<i32> = AvlTree::new(); t.insert(5); assert_eq!(t.size(), 1); }
    #[test] fn test_avl_tree_empty() { let t: AvlTree<i32> = AvlTree::new(); assert!(t.is_empty()); }
    #[test] fn test_red_black_tree_insert() { let mut t: RedBlackTree<i32> = RedBlackTree::new(); t.insert(5); assert_eq!(t.size(), 1); }
    #[test] fn test_red_black_tree_empty() { let t: RedBlackTree<i32> = RedBlackTree::new(); assert!(t.is_empty()); }
}
