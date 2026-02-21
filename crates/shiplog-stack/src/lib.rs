//! Stack data structure implementations for shiplog.
//!
//! This crate provides stack implementations using both array-based and linked list-based
//! approaches for LIFO (Last In, First Out) data management.

use std::collections::LinkedList;

/// A stack implementation using a Vec (array-based).
#[derive(Debug, Clone)]
pub struct ArrayStack<T> {
    data: Vec<T>,
}

impl<T> ArrayStack<T> {
    /// Creates a new empty ArrayStack.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Creates a new ArrayStack with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Pushes an element onto the stack.
    pub fn push(&mut self, value: T) {
        self.data.push(value);
    }

    /// Removes and returns the top element from the stack.
    pub fn pop(&mut self) -> Option<T> {
        self.data.pop()
    }

    /// Returns a reference to the top element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.data.last()
    }

    /// Returns a mutable reference to the top element without removing it.
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.data.last_mut()
    }

    /// Returns true if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Returns the number of elements in the stack.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns the capacity of the underlying vector.
    pub fn capacity(&self) -> usize {
        self.data.capacity()
    }

    /// Clears all elements from the stack.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Returns an iterator over the stack elements.
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data.iter()
    }

    /// Returns a mutable iterator over the stack elements.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.data.iter_mut()
    }
}

impl<T> Default for ArrayStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<Vec<T>> for ArrayStack<T> {
    fn from(vec: Vec<T>) -> Self {
        Self { data: vec }
    }
}

impl<T> From<ArrayStack<T>> for Vec<T> {
    fn from(val: ArrayStack<T>) -> Self {
        val.data
    }
}

/// A node in the linked list stack.
#[derive(Debug, Clone)]
struct Node<T> {
    value: T,
    next: Option<Box<Node<T>>>,
}

/// A stack implementation using a linked list.
#[derive(Debug, Clone)]
pub struct LinkedListStack<T> {
    top: Option<Box<Node<T>>>,
    size: usize,
}

impl<T> LinkedListStack<T> {
    /// Creates a new empty LinkedListStack.
    pub fn new() -> Self {
        Self { top: None, size: 0 }
    }

    /// Pushes an element onto the stack.
    pub fn push(&mut self, value: T) {
        let new_node = Box::new(Node {
            value,
            next: self.top.take(),
        });
        self.top = Some(new_node);
        self.size += 1;
    }

    /// Removes and returns the top element from the stack.
    pub fn pop(&mut self) -> Option<T> {
        self.top.take().map(|node| {
            self.top = node.next;
            self.size -= 1;
            node.value
        })
    }

    /// Returns a reference to the top element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.top.as_ref().map(|node| &node.value)
    }

    /// Returns a mutable reference to the top element without removing it.
    pub fn peek_mut(&mut self) -> Option<&mut T> {
        self.top.as_mut().map(|node| &mut node.value)
    }

    /// Returns true if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.top.is_none()
    }

    /// Returns the number of elements in the stack.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Clears all elements from the stack.
    pub fn clear(&mut self) {
        self.top = None;
        self.size = 0;
    }
}

impl<T> Default for LinkedListStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> From<LinkedList<T>> for LinkedListStack<T> {
    fn from(list: LinkedList<T>) -> Self {
        let mut stack = LinkedListStack::new();
        // LinkedList iterates from front to back, but we want to preserve order
        // So we collect into a vec first, then push in reverse
        let items: Vec<T> = list.into_iter().collect();
        for item in items.into_iter().rev() {
            stack.push(item);
        }
        stack
    }
}

/// A stack implementation using the standard library's LinkedList.
#[derive(Debug, Clone)]
pub struct StdLinkedListStack<T> {
    list: LinkedList<T>,
}

impl<T> StdLinkedListStack<T> {
    /// Creates a new empty StdLinkedListStack.
    pub fn new() -> Self {
        Self {
            list: LinkedList::new(),
        }
    }

    /// Pushes an element onto the stack.
    pub fn push(&mut self, value: T) {
        self.list.push_front(value);
    }

    /// Removes and returns the top element from the stack.
    pub fn pop(&mut self) -> Option<T> {
        self.list.pop_front()
    }

    /// Returns a reference to the top element without removing it.
    pub fn peek(&self) -> Option<&T> {
        self.list.front()
    }

    /// Returns true if the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    /// Returns the number of elements in the stack.
    pub fn len(&self) -> usize {
        self.list.len()
    }

    /// Clears all elements from the stack.
    pub fn clear(&mut self) {
        self.list.clear();
    }
}

impl<T> Default for StdLinkedListStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ArrayStack tests
    #[test]
    fn test_array_stack_new() {
        let stack: ArrayStack<i32> = ArrayStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_array_stack_push_pop() {
        let mut stack = ArrayStack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_array_stack_peek() {
        let mut stack = ArrayStack::new();
        stack.push(42);
        stack.push(99);

        assert_eq!(stack.peek(), Some(&99));
        assert_eq!(stack.len(), 2); // Peek doesn't remove

        *stack.peek_mut().unwrap() = 100;
        assert_eq!(stack.peek(), Some(&100));
    }

    #[test]
    fn test_array_stack_peek_empty() {
        let stack: ArrayStack<i32> = ArrayStack::new();
        assert_eq!(stack.peek(), None);
    }

    #[test]
    fn test_array_stack_pop_empty() {
        let mut stack: ArrayStack<i32> = ArrayStack::new();
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_array_stack_clear() {
        let mut stack = ArrayStack::new();
        stack.push(1);
        stack.push(2);
        stack.clear();

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_array_stack_with_capacity() {
        let stack: ArrayStack<i32> = ArrayStack::with_capacity(100);
        assert!(stack.capacity() >= 100);
    }

    #[test]
    fn test_array_stack_from_vec() {
        let mut stack = ArrayStack::from(vec![1, 2, 3]);
        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Some(3));
    }

    #[test]
    fn test_array_stack_into_vec() {
        let mut stack = ArrayStack::new();
        stack.push(1);
        stack.push(2);
        let vec: Vec<i32> = stack.into();
        assert_eq!(vec, vec![1, 2]);
    }

    // LinkedListStack tests
    #[test]
    fn test_linked_list_stack_new() {
        let stack: LinkedListStack<i32> = LinkedListStack::new();
        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_linked_list_stack_push_pop() {
        let mut stack = LinkedListStack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
        assert!(stack.is_empty());
    }

    #[test]
    fn test_linked_list_stack_peek() {
        let mut stack = LinkedListStack::new();
        stack.push(42);
        stack.push(99);

        assert_eq!(stack.peek(), Some(&99));
        assert_eq!(stack.len(), 2);

        if let Some(val) = stack.peek_mut() {
            *val = 100;
        }
        assert_eq!(stack.peek(), Some(&100));
    }

    #[test]
    fn test_linked_list_stack_peek_empty() {
        let stack: LinkedListStack<i32> = LinkedListStack::new();
        assert_eq!(stack.peek(), None);
    }

    #[test]
    fn test_linked_list_stack_pop_empty() {
        let mut stack: LinkedListStack<i32> = LinkedListStack::new();
        assert_eq!(stack.pop(), None);
    }

    #[test]
    fn test_linked_list_stack_clear() {
        let mut stack = LinkedListStack::new();
        stack.push(1);
        stack.push(2);
        stack.clear();

        assert!(stack.is_empty());
        assert_eq!(stack.len(), 0);
    }

    // StdLinkedListStack tests
    #[test]
    fn test_std_linked_list_stack_new() {
        let stack: StdLinkedListStack<i32> = StdLinkedListStack::new();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_std_linked_list_stack_push_pop() {
        let mut stack = StdLinkedListStack::new();
        stack.push(1);
        stack.push(2);
        stack.push(3);

        assert_eq!(stack.len(), 3);
        assert_eq!(stack.pop(), Some(3));
        assert_eq!(stack.pop(), Some(2));
        assert_eq!(stack.pop(), Some(1));
    }

    #[test]
    fn test_std_linked_list_stack_peek() {
        let mut stack = StdLinkedListStack::new();
        stack.push(42);
        stack.push(99);

        assert_eq!(stack.peek(), Some(&99));
        assert_eq!(stack.len(), 2);
    }

    #[test]
    fn test_std_linked_list_stack_clear() {
        let mut stack = StdLinkedListStack::new();
        stack.push(1);
        stack.push(2);
        stack.clear();

        assert!(stack.is_empty());
    }
}
