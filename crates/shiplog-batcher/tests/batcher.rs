use proptest::prelude::*;
use shiplog_batcher::{BatchConfig, BatchProcessor, Batcher};

// ── Batcher correctness ────────────────────────────────────────────

#[test]
fn batcher_add_below_threshold() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 5,
        flush_timeout_ms: 1000,
    });
    let r = b.add("a".to_string()).unwrap();
    assert!(r.is_none());
    assert_eq!(b.len(), 1);
}

#[test]
fn batcher_auto_flush_at_max_size() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 2,
        flush_timeout_ms: 1000,
    });
    b.add("a".to_string()).unwrap();
    let r = b.add("b".to_string()).unwrap();
    assert!(r.is_some());
    assert_eq!(r.unwrap(), vec!["a".to_string(), "b".to_string()]);
    assert!(b.is_empty());
}

#[test]
fn batcher_manual_flush() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 100,
        flush_timeout_ms: 1000,
    });
    b.add("x".to_string()).unwrap();
    b.add("y".to_string()).unwrap();
    let r = b.flush().unwrap();
    assert_eq!(r.unwrap(), vec!["x".to_string(), "y".to_string()]);
    assert!(b.is_empty());
}

#[test]
fn batcher_flush_empty() {
    let mut b = Batcher::<String>::new(BatchConfig {
        max_size: 10,
        flush_timeout_ms: 1000,
    });
    let r = b.flush().unwrap();
    assert!(r.is_none());
}

#[test]
fn batcher_single_item_max_size_1() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 1,
        flush_timeout_ms: 1000,
    });
    let r = b.add(42i32).unwrap();
    assert_eq!(r, Some(vec![42]));
    assert!(b.is_empty());
}

#[test]
fn batcher_multiple_auto_flushes() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 2,
        flush_timeout_ms: 1000,
    });
    let mut flushed = Vec::new();
    for i in 0..6 {
        if let Some(batch) = b.add(i).unwrap() {
            flushed.push(batch);
        }
    }
    assert_eq!(flushed.len(), 3);
    assert_eq!(flushed[0], vec![0, 1]);
    assert_eq!(flushed[1], vec![2, 3]);
    assert_eq!(flushed[2], vec![4, 5]);
}

#[test]
fn batcher_with_callback() {
    use std::sync::{Arc, Mutex};
    let collected = Arc::new(Mutex::new(Vec::new()));
    let collected_clone = collected.clone();

    let mut b = Batcher::new(BatchConfig {
        max_size: 2,
        flush_timeout_ms: 1000,
    })
    .with_flush_callback(move |batch: Vec<i32>| {
        collected_clone.lock().unwrap().push(batch);
        Ok(())
    });

    b.add(1).unwrap();
    b.add(2).unwrap();
    b.add(3).unwrap();
    b.flush().unwrap();

    let c = collected.lock().unwrap();
    assert_eq!(c.len(), 2);
    assert_eq!(c[0], vec![1, 2]);
    assert_eq!(c[1], vec![3]);
}

// ── BatchProcessor correctness ─────────────────────────────────────

#[test]
fn batch_processor_processes_all() {
    let bp = BatchProcessor::new(BatchConfig {
        max_size: 3,
        flush_timeout_ms: 1000,
    });
    let items: Vec<i32> = (0..10).collect();
    let count = bp.process(&items, |_chunk| Ok(())).unwrap();
    assert_eq!(count, 10);
}

#[test]
fn batch_processor_correct_chunks() {
    let bp = BatchProcessor::new(BatchConfig {
        max_size: 3,
        flush_timeout_ms: 1000,
    });
    let items = vec![1, 2, 3, 4, 5, 6, 7];
    let mut chunks = Vec::new();
    bp.process(&items, |chunk| {
        chunks.push(chunk.to_vec());
        Ok(())
    })
    .unwrap();
    assert_eq!(chunks, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
}

#[test]
fn batch_processor_empty_input() {
    let bp = BatchProcessor::<i32>::new(BatchConfig {
        max_size: 5,
        flush_timeout_ms: 1000,
    });
    let count = bp.process(&[], |_| Ok(())).unwrap();
    assert_eq!(count, 0);
}

#[test]
fn batch_processor_single_item() {
    let bp = BatchProcessor::new(BatchConfig {
        max_size: 10,
        flush_timeout_ms: 1000,
    });
    let count = bp
        .process(&[42], |chunk| {
            assert_eq!(chunk, &[42]);
            Ok(())
        })
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn batch_processor_exact_batch_size() {
    let bp = BatchProcessor::new(BatchConfig {
        max_size: 3,
        flush_timeout_ms: 1000,
    });
    let items = vec![1, 2, 3];
    let mut chunk_count = 0;
    bp.process(&items, |chunk| {
        assert_eq!(chunk.len(), 3);
        chunk_count += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(chunk_count, 1);
}

#[test]
fn batch_processor_propagates_error() {
    let bp = BatchProcessor::new(BatchConfig {
        max_size: 2,
        flush_timeout_ms: 1000,
    });
    let items = vec![1, 2, 3, 4];
    let result = bp.process(&items, |_| anyhow::bail!("processing error"));
    assert!(result.is_err());
}

// ── Large batch edge case ──────────────────────────────────────────

#[test]
fn batcher_large_batch() {
    let mut b = Batcher::new(BatchConfig {
        max_size: 1000,
        flush_timeout_ms: 1000,
    });
    for i in 0..999 {
        assert!(b.add(i).unwrap().is_none());
    }
    assert_eq!(b.len(), 999);
    let r = b.add(999).unwrap();
    assert!(r.is_some());
    assert_eq!(r.unwrap().len(), 1000);
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_batcher_total_items_preserved(
        items in prop::collection::vec(0i32..1000, 0..200),
        max_size in 1usize..50,
    ) {
        let mut b = Batcher::new(BatchConfig { max_size, flush_timeout_ms: 1000 });
        let mut total_flushed = 0usize;
        for item in &items {
            if let Some(batch) = b.add(*item).unwrap() {
                total_flushed += batch.len();
            }
        }
        if let Some(batch) = b.flush().unwrap() {
            total_flushed += batch.len();
        }
        prop_assert_eq!(total_flushed, items.len());
    }

    #[test]
    fn prop_batch_processor_count_matches(
        items in prop::collection::vec(0i32..100, 0..200),
        max_size in 1usize..50,
    ) {
        let bp = BatchProcessor::new(BatchConfig { max_size, flush_timeout_ms: 1000 });
        let count = bp.process(&items, |_| Ok(())).unwrap();
        prop_assert_eq!(count, items.len());
    }

    #[test]
    fn prop_batch_processor_chunks_cover_all(
        items in prop::collection::vec(0i32..100, 0..200),
        max_size in 1usize..50,
    ) {
        let bp = BatchProcessor::new(BatchConfig { max_size, flush_timeout_ms: 1000 });
        let mut all_items = Vec::new();
        bp.process(&items, |chunk| {
            all_items.extend_from_slice(chunk);
            Ok(())
        }).unwrap();
        prop_assert_eq!(all_items, items);
    }

    #[test]
    fn prop_batch_processor_chunk_sizes_bounded(
        items in prop::collection::vec(0i32..100, 1..200),
        max_size in 1usize..50,
    ) {
        let bp = BatchProcessor::new(BatchConfig { max_size, flush_timeout_ms: 1000 });
        bp.process(&items, |chunk| {
            assert!(chunk.len() <= max_size);
            Ok(())
        }).unwrap();
    }
}
