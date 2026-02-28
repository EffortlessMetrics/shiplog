use proptest::prelude::*;
use shiplog_buffer::{Buffer, BufferBuilder, BufferConfig, BufferStrategy, CircularBuffer};

// ── Buffer known-answer tests ───────────────────────────────────────

#[test]
fn buffer_fifo_order() {
    let mut buf: Buffer<i32> = Buffer::new(5);
    for i in 0..5 {
        buf.push(i);
    }
    for i in 0..5 {
        assert_eq!(buf.pop(), Some(i));
    }
    assert_eq!(buf.pop(), None);
}

#[test]
fn buffer_drop_oldest_evicts_front() {
    let mut buf: Buffer<i32> = Buffer::new(3);
    buf.push(1);
    buf.push(2);
    buf.push(3);
    assert!(buf.push(4).is_none()); // drops 1 silently (DropOldest returns None)
    assert_eq!(buf.front(), Some(&2));
    assert_eq!(buf.back(), Some(&4));
}

#[test]
fn buffer_drop_newest_rejects_new() {
    let config = BufferBuilder::new()
        .capacity(2)
        .strategy(BufferStrategy::DropNewest)
        .build();
    let mut buf: Buffer<i32> = Buffer::with_config(&config);
    buf.push(1);
    buf.push(2);
    let rejected = buf.push(3);
    assert_eq!(rejected, Some(3)); // new item returned
    assert_eq!(buf.len(), 2);
    assert_eq!(buf.front(), Some(&1));
}

#[test]
fn buffer_front_back() {
    let mut buf: Buffer<&str> = Buffer::new(3);
    buf.push("a");
    buf.push("b");
    buf.push("c");
    assert_eq!(buf.front(), Some(&"a"));
    assert_eq!(buf.back(), Some(&"c"));
}

#[test]
fn buffer_clear() {
    let mut buf: Buffer<i32> = Buffer::new(5);
    buf.push(1);
    buf.push(2);
    buf.clear();
    assert!(buf.is_empty());
    assert_eq!(buf.len(), 0);
}

#[test]
fn buffer_name() {
    let config = BufferBuilder::new().name("my-buf").build();
    let buf: Buffer<i32> = Buffer::with_config(&config);
    assert_eq!(buf.name(), "my-buf");
}

#[test]
fn buffer_is_full() {
    let mut buf: Buffer<i32> = Buffer::new(2);
    assert!(!buf.is_full());
    buf.push(1);
    assert!(!buf.is_full());
    buf.push(2);
    assert!(buf.is_full());
}

// ── BufferConfig / Builder tests ────────────────────────────────────

#[test]
fn default_config() {
    let config = BufferConfig::default();
    assert_eq!(config.capacity, 100);
    assert_eq!(config.strategy, BufferStrategy::DropOldest);
    assert_eq!(config.name, "buffer");
}

#[test]
fn builder_overrides() {
    let config = BufferBuilder::new()
        .capacity(50)
        .strategy(BufferStrategy::Block)
        .name("test")
        .build();
    assert_eq!(config.capacity, 50);
    assert_eq!(config.strategy, BufferStrategy::Block);
    assert_eq!(config.name, "test");
}

#[test]
fn builder_default_matches_config_default() {
    let from_builder = BufferBuilder::default().build();
    let from_default = BufferConfig::default();
    assert_eq!(from_builder.capacity, from_default.capacity);
    assert_eq!(from_builder.strategy, from_default.strategy);
    assert_eq!(from_builder.name, from_default.name);
}

// ── CircularBuffer known-answer tests ───────────────────────────────

#[test]
fn circular_fifo_order() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(3);
    cb.push(1);
    cb.push(2);
    cb.push(3);
    assert_eq!(cb.pop(), Some(1));
    assert_eq!(cb.pop(), Some(2));
    assert_eq!(cb.pop(), Some(3));
    assert_eq!(cb.pop(), None);
}

#[test]
fn circular_overwrite_returns_old() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(2);
    assert_eq!(cb.push(1), None);
    assert_eq!(cb.push(2), None);
    assert_eq!(cb.push(3), Some(1)); // overwrites 1
    assert_eq!(cb.push(4), Some(2)); // overwrites 2
    assert_eq!(cb.pop(), Some(3));
    assert_eq!(cb.pop(), Some(4));
}

#[test]
fn circular_is_full() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(2);
    assert!(!cb.is_full());
    cb.push(1);
    assert!(!cb.is_full());
    cb.push(2);
    assert!(cb.is_full());
}

#[test]
fn circular_wrap_around_correctness() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(3);
    // Fill and drain multiple times to exercise wrap-around
    for round in 0..5 {
        let base = round * 3;
        cb.push(base);
        cb.push(base + 1);
        cb.push(base + 2);
        assert_eq!(cb.pop(), Some(base));
        assert_eq!(cb.pop(), Some(base + 1));
        assert_eq!(cb.pop(), Some(base + 2));
        assert!(cb.is_empty());
    }
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn buffer_capacity_one() {
    let mut buf: Buffer<i32> = Buffer::new(1);
    buf.push(1);
    assert!(buf.is_full());
    buf.push(2); // drops 1
    assert_eq!(buf.pop(), Some(2));
    assert!(buf.is_empty());
}

#[test]
fn circular_capacity_one() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(1);
    assert_eq!(cb.push(1), None);
    assert!(cb.is_full());
    assert_eq!(cb.push(2), Some(1));
    assert_eq!(cb.pop(), Some(2));
    assert!(cb.is_empty());
}

#[test]
fn pop_from_empty_buffer() {
    let mut buf: Buffer<i32> = Buffer::new(5);
    assert_eq!(buf.pop(), None);
}

#[test]
fn pop_from_empty_circular() {
    let mut cb: CircularBuffer<i32> = CircularBuffer::new(5);
    assert_eq!(cb.pop(), None);
}

#[test]
fn front_back_on_empty() {
    let buf: Buffer<i32> = Buffer::new(5);
    assert_eq!(buf.front(), None);
    assert_eq!(buf.back(), None);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn buffer_len_never_exceeds_capacity(
        cap in 1usize..20,
        items in prop::collection::vec(0i32..1000, 0..50)
    ) {
        let mut buf: Buffer<i32> = Buffer::new(cap);
        for &item in &items {
            buf.push(item);
            prop_assert!(buf.len() <= cap);
        }
    }

    #[test]
    fn buffer_drop_oldest_keeps_latest(
        cap in 1usize..10,
        items in prop::collection::vec(0i32..1000, 1..30)
    ) {
        let mut buf: Buffer<i32> = Buffer::new(cap);
        for &item in &items {
            buf.push(item);
        }
        // The last min(cap, items.len()) items should be present
        let expected_len = cap.min(items.len());
        prop_assert_eq!(buf.len(), expected_len);
        // The back should be the last pushed item
        prop_assert_eq!(buf.back(), Some(items.last().unwrap()));
    }

    #[test]
    fn buffer_drop_newest_keeps_oldest(
        cap in 1usize..10,
        items in prop::collection::vec(0i32..1000, 1..30)
    ) {
        let config = BufferBuilder::new()
            .capacity(cap)
            .strategy(BufferStrategy::DropNewest)
            .build();
        let mut buf: Buffer<i32> = Buffer::with_config(&config);
        for &item in &items {
            buf.push(item);
        }
        let expected_len = cap.min(items.len());
        prop_assert_eq!(buf.len(), expected_len);
        // Front should be the first item
        prop_assert_eq!(buf.front(), Some(&items[0]));
    }

    #[test]
    fn circular_len_never_exceeds_capacity(
        cap in 1usize..20,
        items in prop::collection::vec(0i32..1000, 0..50)
    ) {
        let mut cb: CircularBuffer<i32> = CircularBuffer::new(cap);
        for &item in &items {
            cb.push(item);
            prop_assert!(cb.len() <= cap);
        }
    }

    #[test]
    fn circular_pop_all_drains(
        cap in 1usize..10,
        items in prop::collection::vec(0i32..1000, 1..20)
    ) {
        let mut cb: CircularBuffer<i32> = CircularBuffer::new(cap);
        for &item in &items {
            cb.push(item);
        }
        let len = cb.len();
        for _ in 0..len {
            prop_assert!(cb.pop().is_some());
        }
        prop_assert!(cb.is_empty());
        prop_assert_eq!(cb.pop(), None);
    }

    #[test]
    fn circular_fifo_preserves_order(
        cap in 1usize..20,
        items in prop::collection::vec(0i32..1000, 0..20)
    ) {
        let mut cb: CircularBuffer<i32> = CircularBuffer::new(cap);
        for &item in &items {
            cb.push(item);
        }
        // Items in buffer should be the last min(cap, items.len()) items in order
        let skip = items.len().saturating_sub(cap);
        let expected: Vec<_> = items.iter().skip(skip).copied().collect();
        let mut actual = Vec::new();
        while let Some(v) = cb.pop() {
            actual.push(v);
        }
        prop_assert_eq!(actual, expected);
    }
}
