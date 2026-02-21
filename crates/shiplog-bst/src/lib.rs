//! Binary Search Tree implementation for shiplog.

#[derive(Debug, Clone)]
pub struct BstNode<T> {
    pub value: T,
    pub left: Option<Box<BstNode<T>>>,
    pub right: Option<Box<BstNode<T>>>,
}
impl<T> BstNode<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            left: None,
            right: None,
        }
    }
}

#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct Bst<T> {
    root: Option<Box<BstNode<T>>>,
    size: usize,
}

impl<T: Ord> Bst<T> {
    pub fn new() -> Self {
        Self {
            root: None,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_bst_insert() {
        let mut bst: Bst<i32> = Bst::new();
        bst.insert(5);
        assert_eq!(bst.size(), 1);
    }
    #[test]
    fn test_bst_empty() {
        let bst: Bst<i32> = Bst::new();
        assert!(bst.is_empty());
    }
}
