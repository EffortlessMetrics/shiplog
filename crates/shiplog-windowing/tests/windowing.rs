use proptest::prelude::*;
use shiplog_windowing::*;

// ── Window ────────────────────────────────────────────────────────────

#[test]
fn window_basic_creation() {
    let w = Window::new(100, 200);
    assert_eq!(w.start(), 100);
    assert_eq!(w.end(), 200);
    assert_eq!(w.size(), 100);
}

#[test]
fn window_from_duration() {
    let w = Window::from_duration(1000, 500);
    assert_eq!(w.start(), 1000);
    assert_eq!(w.end(), 1500);
    assert_eq!(w.size(), 500);
}

#[test]
fn window_zero_duration() {
    let w = Window::from_duration(100, 0);
    assert_eq!(w.size(), 0);
    assert!(!w.contains(100)); // [100, 100) is empty
}

#[test]
fn window_contains_start_inclusive() {
    let w = Window::new(10, 20);
    assert!(w.contains(10)); // start is inclusive
}

#[test]
fn window_contains_end_exclusive() {
    let w = Window::new(10, 20);
    assert!(!w.contains(20)); // end is exclusive
}

#[test]
fn window_contains_mid() {
    let w = Window::new(0, 100);
    assert!(w.contains(50));
}

#[test]
fn window_does_not_contain_before_start() {
    let w = Window::new(10, 20);
    assert!(!w.contains(9));
}

#[test]
fn window_does_not_contain_after_end() {
    let w = Window::new(10, 20);
    assert!(!w.contains(21));
}

#[test]
fn window_negative_timestamps() {
    let w = Window::new(-100, 100);
    assert!(w.contains(-100));
    assert!(w.contains(0));
    assert!(w.contains(99));
    assert!(!w.contains(100));
    assert_eq!(w.size(), 200);
}

// ── Window overlaps ───────────────────────────────────────────────────

#[test]
fn window_overlaps_partial() {
    let w1 = Window::new(0, 10);
    let w2 = Window::new(5, 15);
    assert!(w1.overlaps(&w2));
    assert!(w2.overlaps(&w1));
}

#[test]
fn window_overlaps_same() {
    let w = Window::new(0, 10);
    assert!(w.overlaps(&w));
}

#[test]
fn window_no_overlap_adjacent() {
    let w1 = Window::new(0, 10);
    let w2 = Window::new(10, 20);
    assert!(!w1.overlaps(&w2));
    assert!(!w2.overlaps(&w1));
}

#[test]
fn window_no_overlap_disjoint() {
    let w1 = Window::new(0, 5);
    let w2 = Window::new(100, 200);
    assert!(!w1.overlaps(&w2));
}

#[test]
fn window_overlap_containment() {
    let outer = Window::new(0, 100);
    let inner = Window::new(10, 20);
    assert!(outer.overlaps(&inner));
    assert!(inner.overlaps(&outer));
}

// ── Window equality ───────────────────────────────────────────────────

#[test]
fn window_eq() {
    let w1 = Window::new(0, 100);
    let w2 = Window::new(0, 100);
    assert_eq!(w1, w2);
}

#[test]
fn window_ne() {
    let w1 = Window::new(0, 100);
    let w2 = Window::new(0, 200);
    assert_ne!(w1, w2);
}

#[test]
fn window_copy() {
    let w1 = Window::new(0, 100);
    let w2 = w1;
    assert_eq!(w1, w2); // copy semantics
}

// ── TumblingWindow ────────────────────────────────────────────────────

#[test]
fn tumbling_window_current_has_correct_size() {
    let tw = TumblingWindow::new(5000);
    assert_eq!(tw.current().size(), 5000);
}

#[test]
fn tumbling_window_advance() {
    let mut tw = TumblingWindow::new(1000);
    let first = tw.current();
    tw.advance();
    let second = tw.current();
    assert_eq!(second.start(), first.start() + 1000);
    assert_eq!(second.end(), first.end() + 1000);
}

#[test]
fn tumbling_window_for_aligns_to_boundary() {
    let tw = TumblingWindow::new(1000);
    let w = tw.window_for(2500);
    assert_eq!(w.start(), 2000);
    assert_eq!(w.end(), 3000);
}

#[test]
fn tumbling_window_for_exact_boundary() {
    let tw = TumblingWindow::new(1000);
    let w = tw.window_for(3000);
    assert_eq!(w.start(), 3000);
    assert_eq!(w.end(), 4000);
}

#[test]
fn tumbling_window_for_zero() {
    let tw = TumblingWindow::new(1000);
    let w = tw.window_for(0);
    assert_eq!(w.start(), 0);
    assert_eq!(w.end(), 1000);
}

// ── SlidingWindow ─────────────────────────────────────────────────────

#[test]
fn sliding_window_current_size() {
    let sw = SlidingWindow::new(2000, 500);
    assert_eq!(sw.current().size(), 2000);
}

#[test]
fn sliding_window_advance_slides() {
    let mut sw = SlidingWindow::new(2000, 500);
    let first = sw.current();
    sw.advance();
    let second = sw.current();
    assert_eq!(second.start(), first.start() + 500);
}

#[test]
fn sliding_window_for_timestamp() {
    let sw = SlidingWindow::new(1000, 500);
    let w = sw.window_for(1250);
    assert_eq!(w.start(), 1000);
    assert_eq!(w.end(), 2000);
}

#[test]
fn sliding_window_overlapping_windows_count() {
    let sw = SlidingWindow::new(1000, 500);
    let query = Window::new(1000, 1500);
    let windows = sw.overlapping_windows(&query);
    assert!(!windows.is_empty());
    for w in &windows {
        assert!(w.overlaps(&query));
    }
}

// ── SessionWindow ─────────────────────────────────────────────────────

#[test]
fn session_window_single_event() {
    let mut sw = SessionWindow::new(1000);
    sw.add(500);
    assert_eq!(sw.sessions().len(), 1);
    assert_eq!(sw.sessions()[0].start(), 500);
}

#[test]
fn session_window_events_within_gap() {
    let mut sw = SessionWindow::new(1000);
    sw.add(0);
    sw.add(500);
    sw.add(999);
    assert_eq!(sw.sessions().len(), 1);
}

#[test]
fn session_window_events_exceed_gap() {
    let mut sw = SessionWindow::new(100);
    sw.add(0);
    sw.add(200); // gap = 200 > 100
    assert_eq!(sw.sessions().len(), 2);
}

#[test]
fn session_window_default_gap() {
    let sw = SessionWindow::default();
    assert_eq!(sw.sessions().len(), 0);
}

#[test]
fn session_window_clear() {
    let mut sw = SessionWindow::new(1000);
    sw.add(0);
    sw.add(100);
    sw.clear();
    assert_eq!(sw.sessions().len(), 0);
}

#[test]
fn session_window_multiple_sessions() {
    let mut sw = SessionWindow::new(100);
    sw.add(0);
    sw.add(50);
    sw.add(300); // new session
    sw.add(350);
    sw.add(600); // new session
    assert_eq!(sw.sessions().len(), 3);
}

// ── WindowAssigner trait ──────────────────────────────────────────────

#[test]
fn tumbling_assigner_aligns() {
    let assigner = TumblingWindowAssigner::new(1000);
    let w = assigner.assign(1500);
    assert_eq!(w.start(), 1000);
    assert_eq!(w.end(), 2000);
}

#[test]
fn tumbling_assigner_boundary() {
    let assigner = TumblingWindowAssigner::new(1000);
    let w = assigner.assign(2000);
    assert_eq!(w.start(), 2000);
    assert_eq!(w.end(), 3000);
}

#[test]
fn sliding_assigner_basic() {
    let assigner = SlidingWindowAssigner::new(1000, 500);
    let w = assigner.assign(750);
    assert_eq!(w.start(), 500);
    assert_eq!(w.end(), 1500);
}

// ── proptest ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn window_size_is_end_minus_start(start in -10000i64..10000, dur in 0i64..10000) {
        let w = Window::from_duration(start, dur);
        prop_assert_eq!(w.size(), dur);
    }

    #[test]
    fn window_contains_all_within_range(start in 0i64..1000, size in 1i64..1000) {
        let w = Window::new(start, start + size);
        for ts in start..(start + size) {
            prop_assert!(w.contains(ts), "window [{}, {}) should contain {}", start, start + size, ts);
        }
        prop_assert!(!w.contains(start + size));
    }

    #[test]
    fn window_overlaps_symmetric(s1 in 0i64..100, e1 in 100i64..200, s2 in 50i64..150, e2 in 150i64..300) {
        let w1 = Window::new(s1, e1);
        let w2 = Window::new(s2, e2);
        prop_assert_eq!(w1.overlaps(&w2), w2.overlaps(&w1));
    }

    #[test]
    fn tumbling_window_for_always_contains_ts(size_ms in 1i64..10000, ts in 0i64..100000) {
        let tw = TumblingWindow::new(size_ms);
        let w = tw.window_for(ts);
        prop_assert!(w.contains(ts));
        prop_assert_eq!(w.size(), size_ms);
    }

    #[test]
    fn tumbling_assigner_window_contains_ts(size_ms in 1i64..10000, ts in 0i64..100000) {
        let assigner = TumblingWindowAssigner::new(size_ms);
        let w = assigner.assign(ts);
        prop_assert!(w.contains(ts));
    }

    #[test]
    fn window_self_overlaps(start in 0i64..10000, size in 1i64..10000) {
        let w = Window::from_duration(start, size);
        prop_assert!(w.overlaps(&w));
    }

    #[test]
    fn window_adjacent_no_overlap(start in 0i64..10000, size in 1i64..5000) {
        let w1 = Window::new(start, start + size);
        let w2 = Window::new(start + size, start + 2 * size);
        prop_assert!(!w1.overlaps(&w2));
    }
}
