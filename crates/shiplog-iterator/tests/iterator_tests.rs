use shiplog_iterator::*;

#[test]
fn metered_counts_yields() {
    let mut iter = vec![1, 2, 3, 4, 5].into_iter().metered();
    while iter.next().is_some() {}
    assert_eq!(iter.metrics().items_yielded, 5);
}

#[test]
fn metered_empty_iterator() {
    let mut iter = Vec::<i32>::new().into_iter().metered();
    assert_eq!(iter.next(), None);
    assert_eq!(iter.metrics().items_yielded, 0);
}

#[test]
fn skip_while_item_basic() {
    let items: Vec<i32> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .skip_while_item(|x: &i32| *x < 3)
        .collect();
    assert_eq!(items, vec![3, 4, 5]);
}

#[test]
fn skip_while_item_none_skipped() {
    let items: Vec<i32> = vec![5, 4, 3]
        .into_iter()
        .skip_while_item(|x: &i32| *x < 1)
        .collect();
    assert_eq!(items, vec![5, 4, 3]);
}

#[test]
fn skip_while_item_all_skipped() {
    let items: Vec<i32> = vec![1, 2, 3]
        .into_iter()
        .skip_while_item(|x: &i32| *x < 100)
        .collect();
    assert!(items.is_empty());
}

#[test]
fn take_while_item_basic() {
    let items: Vec<i32> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .take_while_item(|x: &i32| *x < 4)
        .collect();
    assert_eq!(items, vec![1, 2, 3]);
}

#[test]
fn take_while_item_all_taken() {
    let items: Vec<i32> = vec![1, 2, 3]
        .into_iter()
        .take_while_item(|x: &i32| *x < 100)
        .collect();
    assert_eq!(items, vec![1, 2, 3]);
}

#[test]
fn take_while_item_none_taken() {
    let items: Vec<i32> = vec![10, 20, 30]
        .into_iter()
        .take_while_item(|x: &i32| *x < 1)
        .collect();
    assert!(items.is_empty());
}

#[test]
fn iterator_config_defaults() {
    let config = IteratorConfig::default();
    assert_eq!(config.batch_size, 10);
    assert_eq!(config.buffer_size, 100);
}

#[test]
fn iterator_builder_fluent() {
    let config = IteratorBuilder::new()
        .batch_size(50)
        .buffer_size(500)
        .build();
    assert_eq!(config.batch_size, 50);
    assert_eq!(config.buffer_size, 500);
}

#[test]
fn iterator_metrics_record() {
    let mut m = IteratorMetrics::new();
    m.record_yield();
    m.record_yield();
    m.record_skip();
    assert_eq!(m.items_yielded, 2);
    assert_eq!(m.items_skipped, 1);
}
