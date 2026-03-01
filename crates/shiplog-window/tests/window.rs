use proptest::prelude::*;
use shiplog_window::*;

// ── WindowConfig defaults ─────────────────────────────────────────────

#[test]
fn window_config_default() {
    let config = WindowConfig::default();
    assert_eq!(config.size, 10);
    assert_eq!(config.step, 1);
    assert_eq!(config.name, "window");
}

#[test]
fn window_config_clone() {
    let config = WindowConfig {
        size: 5,
        step: 2,
        name: "test".to_string(),
    };
    let cloned = config.clone();
    assert_eq!(cloned.size, 5);
    assert_eq!(cloned.step, 2);
    assert_eq!(cloned.name, "test");
}

// ── WindowBuilder ─────────────────────────────────────────────────────

#[test]
fn window_builder_default() {
    let config = WindowBuilder::default().build();
    assert_eq!(config.size, 10);
}

#[test]
fn window_builder_chained() {
    let config = WindowBuilder::new()
        .size(100)
        .step(10)
        .name("custom")
        .build();
    assert_eq!(config.size, 100);
    assert_eq!(config.step, 10);
    assert_eq!(config.name, "custom");
}

// ── SlidingWindow ─────────────────────────────────────────────────────

#[test]
fn sliding_window_new_empty() {
    let w: SlidingWindow<i32> = SlidingWindow::new(5);
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
    assert!(!w.is_full());
    assert_eq!(w.size(), 5);
}

#[test]
fn sliding_window_push_until_full() {
    let mut w: SlidingWindow<i32> = SlidingWindow::new(3);
    w.push(1);
    w.push(2);
    assert!(!w.is_full());
    w.push(3);
    assert!(w.is_full());
    assert_eq!(w.to_vec(), vec![1, 2, 3]);
}

#[test]
fn sliding_window_evicts_oldest() {
    let mut w: SlidingWindow<i32> = SlidingWindow::new(3);
    for i in 1..=5 {
        w.push(i);
    }
    assert_eq!(w.to_vec(), vec![3, 4, 5]);
}

#[test]
fn sliding_window_size_one() {
    let mut w: SlidingWindow<i32> = SlidingWindow::new(1);
    w.push(10);
    assert!(w.is_full());
    assert_eq!(w.to_vec(), vec![10]);
    w.push(20);
    assert_eq!(w.to_vec(), vec![20]);
}

#[test]
fn sliding_window_get_window_refs() {
    let mut w: SlidingWindow<String> = SlidingWindow::new(2);
    w.push("hello".to_string());
    w.push("world".to_string());
    let refs = w.get_window();
    assert_eq!(refs, vec![&"hello".to_string(), &"world".to_string()]);
}

#[test]
fn sliding_window_clear() {
    let mut w: SlidingWindow<i32> = SlidingWindow::new(5);
    w.push(1);
    w.push(2);
    w.clear();
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
}

#[test]
fn sliding_window_map() {
    let mut w: SlidingWindow<i32> = SlidingWindow::new(3);
    w.push(2);
    w.push(4);
    w.push(6);
    let doubled: Vec<i32> = w.map(|x| x * 2);
    assert_eq!(doubled, vec![4, 8, 12]);
}

#[test]
fn sliding_window_name() {
    let config = WindowBuilder::new().name("my-win").size(3).build();
    let w: SlidingWindow<i32> = SlidingWindow::with_config(&config);
    assert_eq!(w.name(), "my-win");
}

#[test]
fn sliding_window_with_step() {
    let w: SlidingWindow<i32> = SlidingWindow::new(5).with_step(2);
    assert_eq!(w.size(), 5);
}

// ── TumblingWindow ────────────────────────────────────────────────────

#[test]
fn tumbling_window_empty() {
    let w: TumblingWindow<i32> = TumblingWindow::new(3);
    assert!(w.is_empty());
    assert_eq!(w.len(), 0);
}

#[test]
fn tumbling_window_emits_at_size() {
    let mut w: TumblingWindow<i32> = TumblingWindow::new(3);
    assert!(w.push(1).is_none());
    assert!(w.push(2).is_none());
    let result = w.push(3);
    assert_eq!(result, Some(vec![1, 2, 3]));
    assert!(w.is_empty());
}

#[test]
fn tumbling_window_resets_after_emit() {
    let mut w: TumblingWindow<i32> = TumblingWindow::new(2);
    assert_eq!(w.push(1), None);
    assert_eq!(w.push(2), Some(vec![1, 2]));
    assert_eq!(w.push(3), None);
    assert_eq!(w.push(4), Some(vec![3, 4]));
}

#[test]
fn tumbling_window_with_name() {
    let w: TumblingWindow<i32> = TumblingWindow::new(5).with_name("test");
    assert!(w.is_empty());
}

#[test]
fn tumbling_window_size_one() {
    let mut w: TumblingWindow<i32> = TumblingWindow::new(1);
    assert_eq!(w.push(42), Some(vec![42]));
    assert!(w.is_empty());
}

// ── WindowStats ───────────────────────────────────────────────────────

#[test]
fn window_stats_default() {
    let stats = WindowStats::new();
    assert_eq!(stats.windows_created, 0);
    assert_eq!(stats.items_processed, 0);
}

#[test]
fn window_stats_record() {
    let mut stats = WindowStats::new();
    stats.record_item();
    stats.record_item();
    stats.record_item();
    stats.record_window();
    assert_eq!(stats.items_processed, 3);
    assert_eq!(stats.windows_created, 1);
}

#[test]
fn window_stats_clone() {
    let mut stats = WindowStats::new();
    stats.record_item();
    let cloned = stats.clone();
    assert_eq!(cloned.items_processed, 1);
}

// ── proptest ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn sliding_window_never_exceeds_size(size in 1usize..50, pushes in 1usize..200) {
        let mut w: SlidingWindow<usize> = SlidingWindow::new(size);
        for i in 0..pushes {
            w.push(i);
            prop_assert!(w.len() <= size);
        }
    }

    #[test]
    fn sliding_window_full_after_enough_pushes(size in 1usize..50, extra in 0usize..50) {
        let mut w: SlidingWindow<usize> = SlidingWindow::new(size);
        for i in 0..(size + extra) {
            w.push(i);
        }
        prop_assert!(w.is_full());
        prop_assert_eq!(w.len(), size);
    }

    #[test]
    fn tumbling_window_emits_at_boundary(size in 1usize..20, count in 0usize..100) {
        let mut w: TumblingWindow<usize> = TumblingWindow::new(size);
        let mut emitted = 0usize;
        for i in 0..count {
            if w.push(i).is_some() {
                emitted += 1;
            }
        }
        prop_assert_eq!(emitted, count / size);
    }

    #[test]
    fn tumbling_window_emit_has_correct_size(size in 1usize..20, count in 0usize..100) {
        let mut w: TumblingWindow<usize> = TumblingWindow::new(size);
        for i in 0..count {
            if let Some(batch) = w.push(i) {
                prop_assert_eq!(batch.len(), size);
            }
        }
    }

    #[test]
    fn sliding_window_to_vec_matches_get_window(ref data in proptest::collection::vec(any::<i32>(), 1..50)) {
        let size = std::cmp::min(data.len(), 10);
        let mut w: SlidingWindow<i32> = SlidingWindow::new(size);
        for &item in data {
            w.push(item);
        }
        let refs: Vec<i32> = w.get_window().iter().map(|&&x| x).collect();
        prop_assert_eq!(refs, w.to_vec());
    }
}
