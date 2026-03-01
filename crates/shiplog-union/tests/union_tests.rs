use shiplog_union::*;

#[test]
fn union_all_includes_duplicates() {
    let result = StreamUnion::new()
        .add_stream(vec![1, 2])
        .add_stream(vec![2, 3])
        .with_mode(UnionMode::All)
        .execute();
    assert_eq!(result, vec![1, 2, 2, 3]);
}

#[test]
fn union_distinct_removes_duplicates() {
    let result = StreamUnion::new()
        .add_stream(vec![1, 2, 3])
        .add_stream(vec![3, 4, 5])
        .with_mode(UnionMode::Distinct)
        .execute();
    assert_eq!(result.len(), 5);
}

#[test]
fn union_keep_first() {
    let result = StreamUnion::new()
        .add_stream(vec![1, 2])
        .add_stream(vec![1, 3])
        .with_mode(UnionMode::KeepFirst)
        .execute();
    assert_eq!(result.iter().filter(|&&x| x == 1).count(), 1);
    assert_eq!(result.len(), 3);
}

#[test]
fn union_keep_last() {
    let result = StreamUnion::new()
        .add_stream(vec![1, 2])
        .add_stream(vec![1, 3])
        .with_mode(UnionMode::KeepLast)
        .execute();
    assert_eq!(result.iter().filter(|&&x| x == 1).count(), 1);
    assert_eq!(result.len(), 3);
}

#[test]
fn union_empty_streams() {
    let result = StreamUnion::<i32>::new()
        .add_stream(vec![])
        .add_stream(vec![])
        .execute();
    assert!(result.is_empty());
}

#[test]
fn union_single_stream() {
    let result = StreamUnion::new().add_stream(vec![1, 2, 3]).execute();
    assert_eq!(result, vec![1, 2, 3]);
}

#[test]
fn union_with_counts() {
    let result = StreamUnion::new()
        .add_stream(vec![1, 2])
        .add_stream(vec![3, 4, 5])
        .execute_with_counts();
    assert_eq!(result.source_counts, vec![2, 3]);
    assert_eq!(result.items.len(), 5);
}

#[test]
fn interleaved_merger_basic() {
    let result = InterleavedMerger::new()
        .add_stream(vec![1, 2, 3])
        .add_stream(vec![4, 5, 6])
        .execute();
    assert_eq!(result, vec![1, 4, 2, 5, 3, 6]);
}

#[test]
fn interleaved_merger_uneven() {
    let result = InterleavedMerger::new()
        .add_stream(vec![1])
        .add_stream(vec![2, 3, 4])
        .execute();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn interleaved_merger_empty() {
    let result = InterleavedMerger::<i32>::new().execute();
    assert!(result.is_empty());
}

#[test]
fn chained_merger_basic() {
    let result = ChainedMerger::new()
        .add_stream(vec![1, 2])
        .add_stream(vec![3, 4])
        .execute();
    assert_eq!(result, vec![1, 2, 3, 4]);
}

#[test]
fn chained_merger_empty() {
    let result = ChainedMerger::<i32>::new().execute();
    assert!(result.is_empty());
}

#[test]
fn stream_union_default() {
    let su = StreamUnion::<i32>::default();
    assert!(su.execute().is_empty());
}

#[test]
fn interleaved_merger_default() {
    let im = InterleavedMerger::<i32>::default();
    assert!(im.execute().is_empty());
}

#[test]
fn chained_merger_default() {
    let cm = ChainedMerger::<i32>::default();
    assert!(cm.execute().is_empty());
}
