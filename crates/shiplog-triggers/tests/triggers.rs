use shiplog_triggers::{
    AlwaysTrigger, AndTrigger, CountTrigger, NeverTrigger, OrTrigger, Trigger, TriggerResult,
    WatermarkTrigger,
};

// --- CountTrigger tests ---

#[test]
fn count_trigger_fires_at_threshold() {
    let mut t = CountTrigger::new(3);
    assert_eq!(t.evaluate(0), TriggerResult::Continue);

    assert!(!t.increment()); // 1
    assert!(!t.increment()); // 2
    assert!(t.increment()); // 3 -> should fire
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn count_trigger_threshold_one() {
    let mut t = CountTrigger::new(1);
    assert!(t.increment());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn count_trigger_reset_clears_state() {
    let mut t = CountTrigger::new(2);
    t.increment();
    t.increment();
    assert_eq!(t.evaluate(0), TriggerResult::Fire);

    t.reset();
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

#[test]
fn count_trigger_continues_after_threshold() {
    let mut t = CountTrigger::new(2);
    t.increment();
    t.increment();
    t.increment(); // past threshold
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

// --- WatermarkTrigger tests ---

#[test]
fn watermark_fires_at_threshold() {
    let mut t = WatermarkTrigger::new(100);
    assert_eq!(t.evaluate(50), TriggerResult::Continue);
    assert_eq!(t.evaluate(99), TriggerResult::Continue);
    assert_eq!(t.evaluate(100), TriggerResult::Fire);
}

#[test]
fn watermark_finishes_after_fire() {
    let mut t = WatermarkTrigger::new(100);
    t.evaluate(100); // Fire
    assert_eq!(t.evaluate(200), TriggerResult::Finish);
}

#[test]
fn watermark_reset_allows_refire() {
    let mut t = WatermarkTrigger::new(100);
    t.evaluate(100); // Fire
    t.reset();
    assert_eq!(t.evaluate(50), TriggerResult::Continue);
    assert_eq!(t.evaluate(100), TriggerResult::Fire);
}

#[test]
fn watermark_fires_exactly_at_threshold() {
    let mut t = WatermarkTrigger::new(0);
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

// --- NeverTrigger tests ---

#[test]
fn never_trigger_never_fires() {
    let mut t = NeverTrigger::new();
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
    assert_eq!(t.evaluate(i64::MAX), TriggerResult::Continue);
    assert_eq!(t.evaluate(i64::MIN), TriggerResult::Continue);
}

#[test]
fn never_trigger_default() {
    let mut t: NeverTrigger = Default::default();
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

#[test]
fn never_trigger_reset_is_noop() {
    let mut t = NeverTrigger::new();
    t.reset();
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

// --- AlwaysTrigger tests ---

#[test]
fn always_trigger_always_fires() {
    let mut t = AlwaysTrigger::new();
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
    assert_eq!(t.evaluate(i64::MAX), TriggerResult::Fire);
    assert_eq!(t.evaluate(i64::MIN), TriggerResult::Fire);
}

#[test]
fn always_trigger_default() {
    let mut t: AlwaysTrigger = Default::default();
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn always_trigger_reset_is_noop() {
    let mut t = AlwaysTrigger::new();
    t.reset();
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

// --- AndTrigger tests ---

#[test]
fn and_both_fire() {
    let mut t = AndTrigger::new(AlwaysTrigger::new(), AlwaysTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn and_one_continues() {
    let mut t = AndTrigger::new(AlwaysTrigger::new(), NeverTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

#[test]
fn and_both_continue() {
    let mut t = AndTrigger::new(NeverTrigger::new(), NeverTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

#[test]
fn and_finish_propagates() {
    let mut wm = WatermarkTrigger::new(10);
    wm.evaluate(10); // Fire
    // Now wm will return Finish
    let mut t = AndTrigger::new(wm, AlwaysTrigger::new());
    assert_eq!(t.evaluate(20), TriggerResult::Finish);
}

#[test]
fn and_trigger_reset() {
    let mut t = AndTrigger::new(WatermarkTrigger::new(10), WatermarkTrigger::new(10));
    t.evaluate(10);
    t.reset();
    assert_eq!(t.evaluate(5), TriggerResult::Continue);
}

// --- OrTrigger tests ---

#[test]
fn or_either_fires() {
    let mut t = OrTrigger::new(AlwaysTrigger::new(), NeverTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn or_both_fire() {
    let mut t = OrTrigger::new(AlwaysTrigger::new(), AlwaysTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn or_neither_fires() {
    let mut t = OrTrigger::new(NeverTrigger::new(), NeverTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Continue);
}

#[test]
fn or_both_finish() {
    let mut wm1 = WatermarkTrigger::new(10);
    let mut wm2 = WatermarkTrigger::new(10);
    wm1.evaluate(10);
    wm2.evaluate(10);
    let mut t = OrTrigger::new(wm1, wm2);
    assert_eq!(t.evaluate(20), TriggerResult::Finish);
}

#[test]
fn or_trigger_reset() {
    let mut t = OrTrigger::new(WatermarkTrigger::new(10), WatermarkTrigger::new(10));
    t.evaluate(10);
    t.reset();
    assert_eq!(t.evaluate(5), TriggerResult::Continue);
}

// --- Composite nesting ---

#[test]
fn nested_and_or() {
    // (Always AND Never) OR Always => Fire
    let inner = AndTrigger::new(AlwaysTrigger::new(), NeverTrigger::new());
    let mut t = OrTrigger::new(inner, AlwaysTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

#[test]
fn nested_or_and() {
    // (Always OR Never) AND Always => Fire
    let inner = OrTrigger::new(AlwaysTrigger::new(), NeverTrigger::new());
    let mut t = AndTrigger::new(inner, AlwaysTrigger::new());
    assert_eq!(t.evaluate(0), TriggerResult::Fire);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn count_trigger_fires_after_n_increments(n in 1usize..100) {
            let mut t = CountTrigger::new(n);
            for _ in 0..n {
                t.increment();
            }
            prop_assert_eq!(t.evaluate(0), TriggerResult::Fire);
        }

        #[test]
        fn count_trigger_continues_before_n(n in 2usize..100, k in 0usize..100) {
            let k = k % (n - 1); // ensure k < n-1
            let mut t = CountTrigger::new(n);
            for _ in 0..=k {
                t.increment();
            }
            if k + 1 < n {
                prop_assert_eq!(t.evaluate(0), TriggerResult::Continue);
            }
        }

        #[test]
        fn watermark_threshold(threshold in 0i64..10000, ts in 0i64..10000) {
            let mut t = WatermarkTrigger::new(threshold);
            let result = t.evaluate(ts);
            if ts >= threshold {
                prop_assert_eq!(result, TriggerResult::Fire);
            } else {
                prop_assert_eq!(result, TriggerResult::Continue);
            }
        }

        #[test]
        fn always_fires_for_any_timestamp(ts in i64::MIN..i64::MAX) {
            let mut t = AlwaysTrigger::new();
            prop_assert_eq!(t.evaluate(ts), TriggerResult::Fire);
        }

        #[test]
        fn never_continues_for_any_timestamp(ts in i64::MIN..i64::MAX) {
            let mut t = NeverTrigger::new();
            prop_assert_eq!(t.evaluate(ts), TriggerResult::Continue);
        }
    }
}
