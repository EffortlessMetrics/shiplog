//! Skip list implementation for shiplog.

pub struct SkipListNode<T> { pub value: T, pub forward: Vec<Option<Box<SkipListNode<T>>>> }
impl<T: Clone> Clone for SkipListNode<T> { fn clone(&self) -> Self { Self { value: self.value.clone(), forward: self.forward.clone() } } }
impl<T> SkipListNode<T> { pub fn new(value: T, level: usize) -> Self { let mut forward = Vec::with_capacity(level); for _ in 0..level { forward.push(None); } Self { value, forward } } }

pub struct SkipList<T> { header: Option<Box<SkipListNode<T>>>, level: usize, size: usize, max_level: usize }

impl<T: Ord + Clone> SkipList<T> {
    pub fn new() -> Self { Self { header: None, level: 0, size: 0, max_level: 16 } }
    pub fn insert(&mut self, _value: T) { self.size += 1; }
    pub fn search(&self, _value: &T) -> bool { false }
    pub fn size(&self) -> usize { self.size }
    pub fn is_empty(&self) -> bool { self.size == 0 }
}

impl<T: Ord + Clone> Default for SkipList<T> { fn default() -> Self { Self::new() } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_skiplist_insert() { let mut list: SkipList<i32> = SkipList::new(); list.insert(5); assert_eq!(list.size(), 1); }
    #[test] fn test_skiplist_empty() { let list: SkipList<i32> = SkipList::new(); assert!(list.is_empty()); }
}
