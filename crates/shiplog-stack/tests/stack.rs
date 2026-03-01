use proptest::prelude::*;
use shiplog_stack::{ArrayStack, LinkedListStack, StdLinkedListStack};

// ── Property tests: ArrayStack ──────────────────────────────────────────────

proptest! {
    #[test]
    fn array_push_pop_is_lifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut stack = ArrayStack::new();
        for v in &values {
            stack.push(*v);
        }
        for v in values.iter().rev() {
            prop_assert_eq!(stack.pop(), Some(*v));
        }
        prop_assert_eq!(stack.pop(), None);
    }

    #[test]
    fn array_len_tracks_operations(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut stack = ArrayStack::new();
        for (i, v) in values.iter().enumerate() {
            stack.push(*v);
            prop_assert_eq!(stack.len(), i + 1);
        }
        for i in (0..values.len()).rev() {
            stack.pop();
            prop_assert_eq!(stack.len(), i);
        }
    }

    #[test]
    fn array_peek_returns_last_pushed(values in proptest::collection::vec(any::<i32>(), 1..100)) {
        let mut stack = ArrayStack::new();
        for v in &values {
            stack.push(*v);
            prop_assert_eq!(stack.peek(), Some(v));
        }
    }

    #[test]
    fn linked_push_pop_is_lifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut stack = LinkedListStack::new();
        for v in &values {
            stack.push(*v);
        }
        for v in values.iter().rev() {
            prop_assert_eq!(stack.pop(), Some(*v));
        }
        prop_assert_eq!(stack.pop(), None);
    }

    #[test]
    fn linked_len_tracks_operations(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut stack = LinkedListStack::new();
        for (i, v) in values.iter().enumerate() {
            stack.push(*v);
            prop_assert_eq!(stack.len(), i + 1);
        }
    }

    #[test]
    fn std_linked_push_pop_is_lifo(values in proptest::collection::vec(any::<i32>(), 0..100)) {
        let mut stack = StdLinkedListStack::new();
        for v in &values {
            stack.push(*v);
        }
        for v in values.iter().rev() {
            prop_assert_eq!(stack.pop(), Some(*v));
        }
        prop_assert_eq!(stack.pop(), None);
    }
}

// ── Edge cases: ArrayStack ──────────────────────────────────────────────────

#[test]
fn array_empty_stack() {
    let mut stack: ArrayStack<i32> = ArrayStack::new();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.peek(), None);
    assert_eq!(stack.pop(), None);
}

#[test]
fn array_single_element() {
    let mut stack = ArrayStack::new();
    stack.push(42);
    assert!(!stack.is_empty());
    assert_eq!(stack.peek(), Some(&42));
    assert_eq!(stack.pop(), Some(42));
    assert!(stack.is_empty());
}

#[test]
fn array_clear() {
    let mut stack = ArrayStack::new();
    stack.push(1);
    stack.push(2);
    stack.clear();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.peek(), None);
}

#[test]
fn array_with_capacity() {
    let stack: ArrayStack<i32> = ArrayStack::with_capacity(100);
    assert!(stack.capacity() >= 100);
    assert!(stack.is_empty());
}

#[test]
fn array_from_vec_and_back() {
    let stack = ArrayStack::from(vec![10, 20, 30]);
    assert_eq!(stack.len(), 3);
    let vec: Vec<i32> = stack.into();
    assert_eq!(vec, vec![10, 20, 30]);
}

#[test]
fn array_peek_mut() {
    let mut stack = ArrayStack::new();
    stack.push(1);
    *stack.peek_mut().unwrap() = 99;
    assert_eq!(stack.peek(), Some(&99));
}

#[test]
fn array_default() {
    let stack: ArrayStack<i32> = ArrayStack::default();
    assert!(stack.is_empty());
}

#[test]
fn array_iter() {
    let mut stack = ArrayStack::new();
    stack.push(1);
    stack.push(2);
    stack.push(3);
    let sum: i32 = stack.iter().sum();
    assert_eq!(sum, 6);
}

#[test]
fn array_iter_mut() {
    let mut stack = ArrayStack::new();
    stack.push(1);
    stack.push(2);
    for v in stack.iter_mut() {
        *v *= 10;
    }
    assert_eq!(stack.pop(), Some(20));
    assert_eq!(stack.pop(), Some(10));
}

// ── Edge cases: LinkedListStack ─────────────────────────────────────────────

#[test]
fn linked_empty_stack() {
    let mut stack: LinkedListStack<i32> = LinkedListStack::new();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.peek(), None);
    assert_eq!(stack.pop(), None);
}

#[test]
fn linked_single_element() {
    let mut stack = LinkedListStack::new();
    stack.push(42);
    assert!(!stack.is_empty());
    assert_eq!(stack.peek(), Some(&42));
    assert_eq!(stack.pop(), Some(42));
    assert!(stack.is_empty());
}

#[test]
fn linked_clear() {
    let mut stack = LinkedListStack::new();
    stack.push(1);
    stack.push(2);
    stack.clear();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
}

#[test]
fn linked_peek_mut() {
    let mut stack = LinkedListStack::new();
    stack.push(1);
    *stack.peek_mut().unwrap() = 99;
    assert_eq!(stack.peek(), Some(&99));
}

#[test]
fn linked_default() {
    let stack: LinkedListStack<i32> = LinkedListStack::default();
    assert!(stack.is_empty());
}

#[test]
fn linked_from_linked_list() {
    let mut list = std::collections::LinkedList::new();
    list.push_back(1);
    list.push_back(2);
    list.push_back(3);
    let mut stack = LinkedListStack::from(list);
    // Front of LinkedList becomes top of stack
    assert_eq!(stack.pop(), Some(1));
    assert_eq!(stack.pop(), Some(2));
    assert_eq!(stack.pop(), Some(3));
}

// ── Edge cases: StdLinkedListStack ──────────────────────────────────────────

#[test]
fn std_linked_empty() {
    let mut stack: StdLinkedListStack<i32> = StdLinkedListStack::new();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.peek(), None);
    assert_eq!(stack.pop(), None);
}

#[test]
fn std_linked_single_element() {
    let mut stack = StdLinkedListStack::new();
    stack.push(42);
    assert_eq!(stack.peek(), Some(&42));
    assert_eq!(stack.pop(), Some(42));
}

#[test]
fn std_linked_clear() {
    let mut stack = StdLinkedListStack::new();
    stack.push(1);
    stack.push(2);
    stack.clear();
    assert!(stack.is_empty());
}

#[test]
fn std_linked_default() {
    let stack: StdLinkedListStack<i32> = StdLinkedListStack::default();
    assert!(stack.is_empty());
}

// ── Stress tests ────────────────────────────────────────────────────────────

#[test]
fn stress_array_stack() {
    let mut stack = ArrayStack::new();
    for i in 0..10_000 {
        stack.push(i);
    }
    assert_eq!(stack.len(), 10_000);
    for i in (0..10_000).rev() {
        assert_eq!(stack.pop(), Some(i));
    }
}

#[test]
fn stress_linked_stack() {
    let mut stack = LinkedListStack::new();
    for i in 0..10_000 {
        stack.push(i);
    }
    assert_eq!(stack.len(), 10_000);
    for i in (0..10_000).rev() {
        assert_eq!(stack.pop(), Some(i));
    }
}
