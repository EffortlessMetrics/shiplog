use chrono::Duration;
use proptest::prelude::*;
use shiplog_leakybucket::*;

// ── Correctness ─────────────────────────────────────────────────────

#[test]
fn bucket_accepts_up_to_capacity() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(3, 1, Duration::seconds(60)));
    assert!(b.try_add("k"));
    assert!(b.try_add("k"));
    assert!(b.try_add("k"));
    assert!(!b.try_add("k"));
}

#[test]
fn separate_keys_have_independent_buckets() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(1, 1, Duration::seconds(60)));
    assert!(b.try_add("a"));
    assert!(!b.try_add("a"));
    assert!(b.try_add("b"));
}

#[test]
fn current_level_tracks_additions() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(10, 1, Duration::seconds(60)));
    assert_eq!(b.current_level("k"), 0);
    b.try_add("k");
    b.try_add("k");
    assert_eq!(b.current_level("k"), 2);
}

#[test]
fn remaining_capacity_decreases() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(5, 1, Duration::seconds(60)));
    assert_eq!(b.remaining_capacity("k"), 5);
    b.try_add("k");
    assert_eq!(b.remaining_capacity("k"), 4);
}

#[test]
fn reset_clears_specific_key() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(1, 1, Duration::seconds(60)));
    b.try_add("k");
    assert!(!b.try_add("k"));
    b.reset("k");
    assert!(b.try_add("k"));
}

#[test]
fn clear_removes_all_keys() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(1, 1, Duration::seconds(60)));
    b.try_add("a");
    b.try_add("b");
    assert_eq!(b.len(), 2);
    b.clear();
    assert!(b.is_empty());
}

#[test]
fn len_and_is_empty() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(10, 1, Duration::seconds(60)));
    assert!(b.is_empty());
    assert_eq!(b.len(), 0);
    b.try_add("k");
    assert!(!b.is_empty());
    assert_eq!(b.len(), 1);
}

// ── Leak behaviour (time-dependent) ─────────────────────────────────

#[test]
fn bucket_leaks_over_time() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(5, 2, Duration::milliseconds(50)));
    for _ in 0..5 {
        assert!(b.try_add("k"));
    }
    assert!(!b.try_add("k"));

    std::thread::sleep(std::time::Duration::from_millis(120));

    // After ~2 intervals (100 ms), leaked 4 items → level should be 1
    assert!(b.try_add("k"));
}

// ── Preset configs ──────────────────────────────────────────────────

#[test]
fn strict_config_values() {
    let c = LeakyBucketConfig::strict();
    assert_eq!(c.capacity, 10);
    assert_eq!(c.leak_rate, 1);
}

#[test]
fn lenient_config_values() {
    let c = LeakyBucketConfig::lenient();
    assert_eq!(c.capacity, 100);
    assert_eq!(c.leak_rate, 10);
}

#[test]
fn default_config_is_lenient() {
    let c = LeakyBucketConfig::default();
    assert_eq!(c.capacity, 100);
}

// ── Error type ──────────────────────────────────────────────────────

#[test]
fn error_display() {
    let e = LeakyBucketError::new("full", "user1");
    let s = format!("{e}");
    assert!(s.contains("user1"));
    assert!(s.contains("full"));
}

// ── Edge cases ──────────────────────────────────────────────────────

#[test]
fn zero_capacity_always_rejects() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(0, 1, Duration::seconds(1)));
    assert!(!b.try_add("k"));
}

#[test]
fn zero_leak_rate_never_leaks() {
    let mut b = LeakyBucket::new(LeakyBucketConfig::new(2, 0, Duration::seconds(1)));
    b.try_add("k");
    b.try_add("k");
    assert!(!b.try_add("k"));

    std::thread::sleep(std::time::Duration::from_millis(50));
    // Still full because leak_rate is 0
    assert!(!b.try_add("k"));
}

#[test]
fn remaining_capacity_for_unknown_key_is_full() {
    let b = LeakyBucket::new(LeakyBucketConfig::new(10, 1, Duration::seconds(1)));
    assert_eq!(b.remaining_capacity("nobody"), 10);
}

// ── Property tests ──────────────────────────────────────────────────

proptest! {
    #[test]
    fn never_exceeds_capacity(cap in 1u64..100, attempts in 1usize..300) {
        let mut b = LeakyBucket::new(LeakyBucketConfig::new(cap, 0, Duration::seconds(600)));
        let mut accepted = 0usize;
        for _ in 0..attempts {
            if b.try_add("k") {
                accepted += 1;
            }
        }
        prop_assert!(accepted as u64 <= cap);
    }

    #[test]
    fn remaining_plus_level_equals_capacity(cap in 1u64..100, adds in 0usize..100) {
        let mut b = LeakyBucket::new(LeakyBucketConfig::new(cap, 0, Duration::seconds(600)));
        for _ in 0..adds {
            b.try_add("k");
        }
        let level = b.current_level("k");
        let rem = b.remaining_capacity("k");
        prop_assert_eq!(level + rem, cap);
    }
}
