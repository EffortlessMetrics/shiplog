use chrono::Duration;
use shiplog_bucket::*;

#[test]
fn bucket_consume_within_capacity() {
    let config = TokenBucketConfig::new(5, 1, Duration::seconds(1));
    let mut bucket = TokenBucket::new(config);
    assert!(bucket.try_consume("k", 3));
    assert!(bucket.try_consume("k", 2));
    assert!(!bucket.try_consume("k", 1));
}

#[test]
fn bucket_separate_keys() {
    let config = TokenBucketConfig::new(2, 1, Duration::seconds(1));
    let mut bucket = TokenBucket::new(config);
    assert!(bucket.try_consume("a", 2));
    assert!(!bucket.try_consume("a", 1));
    assert!(bucket.try_consume("b", 2));
}

#[test]
fn bucket_available_tokens() {
    let config = TokenBucketConfig::new(10, 1, Duration::seconds(1));
    let mut bucket = TokenBucket::new(config);
    assert_eq!(bucket.available_tokens("k"), 10);
    bucket.try_consume("k", 3);
    assert_eq!(bucket.available_tokens("k"), 7);
}

#[test]
fn bucket_reset() {
    let config = TokenBucketConfig::new(2, 1, Duration::seconds(1));
    let mut bucket = TokenBucket::new(config);
    bucket.try_consume("k", 2);
    assert!(!bucket.try_consume("k", 1));
    bucket.reset("k");
    assert!(bucket.try_consume("k", 1));
}

#[test]
fn bucket_clear() {
    let config = TokenBucketConfig::new(1, 1, Duration::seconds(1));
    let mut bucket = TokenBucket::new(config);
    bucket.try_consume("a", 1);
    bucket.try_consume("b", 1);
    assert_eq!(bucket.len(), 2);
    bucket.clear();
    assert!(bucket.is_empty());
}

#[test]
fn bucket_presets() {
    let strict = TokenBucketConfig::strict();
    assert_eq!(strict.capacity, 10);
    let lenient = TokenBucketConfig::lenient();
    assert_eq!(lenient.capacity, 100);
}

#[test]
fn bucket_error_display() {
    let err = TokenBucketError::new("exhausted", "user1");
    assert!(err.to_string().contains("user1"));
    assert!(err.to_string().contains("exhausted"));
}

#[test]
fn bucket_refill() {
    let config = TokenBucketConfig::new(3, 2, Duration::milliseconds(50));
    let mut bucket = TokenBucket::new(config);
    assert!(bucket.try_consume("k", 3));
    assert!(!bucket.try_consume("k", 1));
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(bucket.try_consume("k", 1));
}
