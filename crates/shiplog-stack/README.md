# shiplog-stack

Stack data structure implementations (Array, Linked List) for shiplog.

## Overview

This crate provides stack implementations using both array-based and linked list-based approaches for LIFO (Last In, First Out) data management.

## Features

- **ArrayStack**: A stack implementation using a Vec (array-based)
- **LinkedListStack**: A custom linked list-based stack implementation
- **StdLinkedListStack**: A stack implementation using std::collections::LinkedList

## Usage

```rust
use shiplog_stack::{ArrayStack, LinkedListStack};

// Array-based stack
let mut stack = ArrayStack::new();
stack.push(1);
stack.push(2);
stack.push(3);

println!("Top: {:?}", stack.pop()); // Some(3)

// Linked list-based stack
let mut ll_stack = LinkedListStack::new();
ll_stack.push("a");
ll_stack.push("b");
println!("Top: {:?}", ll_stack.pop()); // Some("b")
```

## License

MIT OR Apache-2.0
