use shiplog_sink::{BufferSink, FilterSink, TransformSink};

// ── BufferSink ────────────────────────────────────────────────────

#[test]
fn buffer_sink_default_capacity() {
    let sink: BufferSink<i32> = BufferSink::default();
    assert!(sink.is_empty());
    assert_eq!(sink.len(), 0);
}

#[test]
fn buffer_sink_push_within_capacity() {
    let mut sink = BufferSink::new(5);
    for i in 0..5 {
        sink.push(i);
    }
    assert_eq!(sink.len(), 5);
}

#[test]
fn buffer_sink_evicts_oldest_on_overflow() {
    let mut sink = BufferSink::new(3);
    sink.push(1);
    sink.push(2);
    sink.push(3);
    sink.push(4);
    assert_eq!(sink.drain(), vec![2, 3, 4]);
}

#[test]
fn buffer_sink_many_evictions() {
    let mut sink = BufferSink::new(2);
    for i in 0..100 {
        sink.push(i);
    }
    assert_eq!(sink.len(), 2);
    assert_eq!(sink.drain(), vec![98, 99]);
}

#[test]
fn buffer_sink_capacity_one() {
    let mut sink = BufferSink::new(1);
    sink.push(10);
    sink.push(20);
    sink.push(30);
    assert_eq!(sink.len(), 1);
    assert_eq!(sink.drain(), vec![30]);
}

#[test]
fn buffer_sink_drain_empties() {
    let mut sink = BufferSink::new(5);
    sink.push(1);
    sink.push(2);
    let _ = sink.drain();
    assert!(sink.is_empty());
}

#[test]
fn buffer_sink_drain_twice() {
    let mut sink = BufferSink::new(5);
    sink.push(1);
    let first = sink.drain();
    let second = sink.drain();
    assert_eq!(first, vec![1]);
    assert!(second.is_empty());
}

#[test]
fn buffer_sink_clear() {
    let mut sink = BufferSink::new(5);
    sink.push(1);
    sink.push(2);
    sink.clear();
    assert!(sink.is_empty());
    assert_eq!(sink.len(), 0);
}

#[test]
fn buffer_sink_string_items() {
    let mut sink = BufferSink::new(3);
    sink.push("hello".to_string());
    sink.push("世界".to_string());
    sink.push("".to_string());
    assert_eq!(sink.len(), 3);
    let drained = sink.drain();
    assert_eq!(drained, vec!["hello", "世界", ""]);
}

// ── TransformSink ─────────────────────────────────────────────────

#[test]
fn transform_sink_identity() {
    let mut sink = TransformSink::new(|x: i32| x);
    sink.push(42);
    assert_eq!(sink.drain(), vec![42]);
}

#[test]
fn transform_sink_doubles() {
    let mut sink = TransformSink::new(|x: i32| x * 2);
    sink.push(1);
    sink.push(2);
    sink.push(3);
    assert_eq!(sink.drain(), vec![2, 4, 6]);
}

#[test]
fn transform_sink_string_transform() {
    let mut sink = TransformSink::new(|s: String| s.to_uppercase());
    sink.push("hello".to_string());
    sink.push("world".to_string());
    assert_eq!(sink.drain(), vec!["HELLO", "WORLD"]);
}

#[test]
fn transform_sink_empty_drain() {
    let sink = TransformSink::new(|x: i32| x);
    assert!(sink.is_empty());
}

#[test]
fn transform_sink_multiple_drains() {
    let mut sink = TransformSink::new(|x: i32| x + 10);
    sink.push(1);
    assert_eq!(sink.drain(), vec![11]);
    assert!(sink.is_empty());
    sink.push(2);
    assert_eq!(sink.drain(), vec![12]);
}

// ── FilterSink ────────────────────────────────────────────────────

#[test]
fn filter_sink_even_numbers() {
    let mut sink = FilterSink::new(|x: &i32| x % 2 == 0);
    for i in 0..10 {
        sink.push(i);
    }
    assert_eq!(sink.drain(), vec![0, 2, 4, 6, 8]);
}

#[test]
fn filter_sink_all_pass() {
    let mut sink = FilterSink::new(|_: &i32| true);
    sink.push(1);
    sink.push(2);
    assert_eq!(sink.drain(), vec![1, 2]);
}

#[test]
fn filter_sink_none_pass() {
    let mut sink = FilterSink::new(|_: &i32| false);
    sink.push(1);
    sink.push(2);
    assert!(sink.is_empty());
}

#[test]
fn filter_sink_empty_string_filter() {
    let mut sink = FilterSink::new(|s: &String| !s.is_empty());
    sink.push("hello".to_string());
    sink.push("".to_string());
    sink.push("world".to_string());
    assert_eq!(sink.drain(), vec!["hello", "world"]);
}

#[test]
fn filter_sink_drain_resets() {
    let mut sink = FilterSink::new(|x: &i32| *x > 0);
    sink.push(1);
    sink.push(2);
    assert_eq!(sink.drain(), vec![1, 2]);
    assert!(sink.is_empty());
    sink.push(3);
    assert_eq!(sink.drain(), vec![3]);
}

// ── Property tests ────────────────────────────────────────────────

mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn buffer_sink_never_exceeds_capacity(cap in 1..100usize, items in prop::collection::vec(any::<i32>(), 0..200)) {
            let mut sink = BufferSink::new(cap);
            for item in &items {
                sink.push(*item);
            }
            prop_assert!(sink.len() <= cap);
        }

        #[test]
        fn buffer_sink_drain_returns_all(items in prop::collection::vec(any::<i32>(), 0..50)) {
            let mut sink = BufferSink::new(1000);
            for item in &items {
                sink.push(*item);
            }
            let drained = sink.drain();
            prop_assert_eq!(drained.len(), items.len());
            prop_assert_eq!(drained, items);
        }

        #[test]
        fn transform_sink_identity_roundtrip(items in prop::collection::vec(any::<i32>(), 0..100)) {
            let mut sink = TransformSink::new(|x: i32| x);
            for item in &items {
                sink.push(*item);
            }
            prop_assert_eq!(sink.drain(), items);
        }

        #[test]
        fn filter_sink_all_true_preserves(items in prop::collection::vec(any::<i32>(), 0..100)) {
            let mut sink = FilterSink::new(|_: &i32| true);
            for item in &items {
                sink.push(*item);
            }
            prop_assert_eq!(sink.drain(), items);
        }

        #[test]
        fn filter_sink_all_false_empties(items in prop::collection::vec(any::<i32>(), 0..100)) {
            let mut sink = FilterSink::new(|_: &i32| false);
            for item in &items {
                sink.push(*item);
            }
            prop_assert!(sink.is_empty());
        }

        #[test]
        fn buffer_sink_last_n_items(cap in 1..50usize, items in prop::collection::vec(any::<i32>(), 0..200)) {
            let mut sink = BufferSink::new(cap);
            for item in &items {
                sink.push(*item);
            }
            let drained = sink.drain();
            if items.len() <= cap {
                prop_assert_eq!(drained, items);
            } else {
                let expected: Vec<i32> = items[items.len() - cap..].to_vec();
                prop_assert_eq!(drained, expected);
            }
        }
    }
}
