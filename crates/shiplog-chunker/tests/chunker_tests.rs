use shiplog_chunker::*;

#[test]
fn chunk_basic() {
    let chunks: Vec<Vec<i32>> = vec![1, 2, 3, 4, 5, 6, 7].into_iter().chunk(3).collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], vec![1, 2, 3]);
    assert_eq!(chunks[1], vec![4, 5, 6]);
    assert_eq!(chunks[2], vec![7]);
}

#[test]
fn chunk_exact_division() {
    let chunks: Vec<Vec<i32>> = vec![1, 2, 3, 4].into_iter().chunk(2).collect();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], vec![1, 2]);
    assert_eq!(chunks[1], vec![3, 4]);
}

#[test]
fn chunk_empty_input() {
    let chunks: Vec<Vec<i32>> = Vec::<i32>::new().into_iter().chunk(3).collect();
    assert!(chunks.is_empty());
}

#[test]
fn chunk_size_larger_than_input() {
    let chunks: Vec<Vec<i32>> = vec![1, 2].into_iter().chunk(10).collect();
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0], vec![1, 2]);
}

#[test]
fn chunk_size_one() {
    let chunks: Vec<Vec<i32>> = vec![1, 2, 3].into_iter().chunk(1).collect();
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], vec![1]);
    assert_eq!(chunks[1], vec![2]);
    assert_eq!(chunks[2], vec![3]);
}

#[test]
fn chunk_overlap_produces_chunks() {
    let chunks: Vec<Vec<i32>> = vec![1, 2, 3, 4, 5]
        .into_iter()
        .chunk_overlap(3, 1)
        .collect();
    assert!(!chunks.is_empty());
    assert_eq!(chunks[0].len(), 3);
}

#[test]
fn chunk_config_defaults() {
    let config = ChunkConfig::default();
    assert_eq!(config.chunk_size, 100);
    assert_eq!(config.overlap, 0);
}

#[test]
fn chunk_builder_fluent() {
    let config = ChunkBuilder::new().chunk_size(50).overlap(5).build();
    assert_eq!(config.chunk_size, 50);
    assert_eq!(config.overlap, 5);
}
