use proptest::prelude::*;
use shiplog_processor::{BatchProcessor, Pipeline, Processor, StatefulProcessor};

// ── Correctness tests ──────────────────────────────────────────────

#[test]
fn processor_known_transforms() {
    let double = Processor::new("double", |x: i32| x * 2);
    assert_eq!(double.process(0), 0);
    assert_eq!(double.process(1), 2);
    assert_eq!(double.process(-3), -6);
    assert_eq!(double.process(i32::MAX / 2), i32::MAX / 2 * 2);
}

#[test]
fn processor_name_preserved() {
    let p = Processor::new("my-proc", |x: u8| x);
    assert_eq!(p.name(), "my-proc");
}

#[test]
fn processor_type_conversion() {
    let to_string = Processor::new("stringify", |x: i32| x.to_string());
    assert_eq!(to_string.process(42), "42");
}

// ── BatchProcessor tests ───────────────────────────────────────────

#[test]
fn batch_processor_empty_input() {
    let bp: BatchProcessor<i32> = BatchProcessor::new(10);
    let result = bp.process_batch(&[], |&x| x + 1);
    assert!(result.is_empty());
}

#[test]
fn batch_processor_single_item() {
    let bp: BatchProcessor<i32> = BatchProcessor::new(1);
    let result = bp.process_batch(&[42], |&x| x * 2);
    assert_eq!(result, vec![84]);
}

#[test]
fn batch_processor_large_batch() {
    let bp: BatchProcessor<i32> = BatchProcessor::new(1000);
    let items: Vec<i32> = (0..1000).collect();
    let result = bp.process_batch(&items, |&x| x + 1);
    assert_eq!(result.len(), 1000);
    assert_eq!(result[0], 1);
    assert_eq!(result[999], 1000);
}

#[test]
fn batch_processor_default_name() {
    let bp: BatchProcessor<i32> = BatchProcessor::new(5);
    assert_eq!(bp.name(), "batch-processor");
}

// ── StatefulProcessor tests ────────────────────────────────────────

#[test]
fn stateful_processor_accumulates_state() {
    let mut sp = StatefulProcessor::new(0i64, |state, item: i64| (state + item, item));
    sp.process(10);
    sp.process(20);
    sp.process(30);
    assert_eq!(sp.state(), &60);
}

#[test]
fn stateful_processor_reset_clears_state() {
    let mut sp = StatefulProcessor::new(0, |state: i32, item: i32| (state + item, item));
    sp.process(100);
    sp.reset(0);
    assert_eq!(sp.state(), &0);
    sp.process(5);
    assert_eq!(sp.state(), &5);
}

#[test]
fn stateful_processor_empty_sequence() {
    let sp = StatefulProcessor::new(42, |state: i32, item: i32| (state + item, item));
    assert_eq!(sp.state(), &42);
}

// ── Pipeline tests ─────────────────────────────────────────────────

#[test]
fn pipeline_empty_is_identity() {
    let p: Pipeline<i32> = Pipeline::new();
    assert_eq!(p.execute(99), 99);
    assert!(p.is_empty());
    assert_eq!(p.len(), 0);
}

#[test]
fn pipeline_single_stage() {
    let p = Pipeline::<i32>::new().add_stage(|x| x + 1);
    assert_eq!(p.execute(0), 1);
    assert_eq!(p.len(), 1);
}

#[test]
fn pipeline_composition_order() {
    // (x+1)*3 - 2
    let p = Pipeline::<i32>::new()
        .add_stage(|x| x + 1)
        .add_stage(|x| x * 3)
        .add_stage(|x| x - 2);
    assert_eq!(p.execute(0), 1); // (0+1)*3-2 = 1
    assert_eq!(p.execute(5), 16); // (5+1)*3-2 = 16
}

#[test]
fn pipeline_with_name() {
    let p = Pipeline::<i32>::new().with_name("my-pipe");
    assert_eq!(p.name(), "my-pipe");
}

#[test]
fn pipeline_default() {
    let p: Pipeline<i32> = Pipeline::default();
    assert!(p.is_empty());
    assert_eq!(p.execute(7), 7);
}

// ── Composition: Pipeline + Processor ──────────────────────────────

#[test]
fn pipeline_composed_with_processor() {
    let pre = Processor::new("pre", |x: i32| x.abs());
    let pipeline = Pipeline::<i32>::new()
        .add_stage(|x| x * 2)
        .add_stage(|x| x + 10);

    let input = -5;
    let mid = pre.process(input);
    let result = pipeline.execute(mid);
    assert_eq!(result, 20); // abs(-5)*2+10 = 20
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_identity_processor_returns_input(x: i32) {
        let id = Processor::new("id", |v: i32| v);
        prop_assert_eq!(id.process(x), x);
    }

    #[test]
    fn prop_empty_pipeline_is_identity(x in -10000i32..10000) {
        let p: Pipeline<i32> = Pipeline::new();
        prop_assert_eq!(p.execute(x), x);
    }

    #[test]
    fn prop_batch_processor_preserves_length(items in prop::collection::vec(any::<i32>(), 0..200)) {
        let bp: BatchProcessor<i32> = BatchProcessor::new(50);
        let result = bp.process_batch(&items, |&x| x);
        prop_assert_eq!(result.len(), items.len());
    }

    #[test]
    fn prop_pipeline_stages_compose(x in -1000i32..1000, a in -100i32..100, b in -100i32..100) {
        let p = Pipeline::<i32>::new()
            .add_stage(move |v| v.saturating_add(a))
            .add_stage(move |v| v.saturating_add(b));
        let expected = x.saturating_add(a).saturating_add(b);
        prop_assert_eq!(p.execute(x), expected);
    }

    #[test]
    fn prop_stateful_processor_count_matches(
        items in prop::collection::vec(1i32..100, 0..50)
    ) {
        let mut sp = StatefulProcessor::new(0i32, |state, item: i32| (state + 1, item));
        for &i in &items {
            sp.process(i);
        }
        prop_assert_eq!(*sp.state(), items.len() as i32);
    }
}
