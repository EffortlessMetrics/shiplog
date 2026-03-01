//! Integration tests for shiplog-random.

use proptest::prelude::*;
use shiplog_random::*;

// ── Basic range / bound tests ───────────────────────────────────

#[test]
fn random_in_range_bounds() {
    for _ in 0..200 {
        let val = random_in_range(1, 10);
        assert!((1..=10).contains(&val));
    }
}

#[test]
fn random_in_range_single_value() {
    for _ in 0..20 {
        assert_eq!(random_in_range(5, 5), 5);
    }
}

#[test]
fn random_float_bounds() {
    for _ in 0..200 {
        let val = random_float();
        assert!((0.0..1.0).contains(&val));
    }
}

#[test]
fn random_bool_both_values() {
    let mut seen_true = false;
    let mut seen_false = false;
    for _ in 0..200 {
        if random_bool() {
            seen_true = true;
        } else {
            seen_false = true;
        }
    }
    assert!(seen_true && seen_false);
}

// ── Character generation tests ──────────────────────────────────

#[test]
fn random_uppercase_char_range() {
    for _ in 0..100 {
        let c = random_uppercase_char();
        assert!(c.is_ascii_uppercase(), "Expected uppercase, got '{}'", c);
    }
}

#[test]
fn random_lowercase_char_range() {
    for _ in 0..100 {
        let c = random_lowercase_char();
        assert!(c.is_ascii_lowercase(), "Expected lowercase, got '{}'", c);
    }
}

#[test]
fn random_alphanumeric_char_range() {
    for _ in 0..100 {
        let c = random_alphanumeric_char();
        assert!(
            c.is_ascii_alphanumeric(),
            "Expected alphanumeric, got '{}'",
            c
        );
    }
}

// ── String generation tests ─────────────────────────────────────

#[test]
fn random_string_length() {
    assert_eq!(random_string(0).len(), 0);
    assert_eq!(random_string(1).len(), 1);
    assert_eq!(random_string(100).len(), 100);
}

#[test]
fn random_string_content() {
    let s = random_string(50);
    assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
}

#[test]
fn random_lowercase_string_content() {
    let s = random_lowercase_string(50);
    assert_eq!(s.len(), 50);
    assert!(s.chars().all(|c| c.is_ascii_lowercase()));
}

#[test]
fn random_uppercase_string_content() {
    let s = random_uppercase_string(50);
    assert_eq!(s.len(), 50);
    assert!(s.chars().all(|c| c.is_ascii_uppercase()));
}

#[test]
fn random_string_empty() {
    assert_eq!(random_string(0), "");
    assert_eq!(random_lowercase_string(0), "");
    assert_eq!(random_uppercase_string(0), "");
}

// ── Bytes generation tests ──────────────────────────────────────

#[test]
fn random_bytes_length() {
    assert_eq!(random_bytes(0).len(), 0);
    assert_eq!(random_bytes(16).len(), 16);
    assert_eq!(random_bytes(256).len(), 256);
}

// ── ThreadRng tests ─────────────────────────────────────────────

#[test]
fn thread_rng_range_bounds() {
    for _ in 0..100 {
        let val = ThreadRng::range(0, 100);
        assert!((0..=100).contains(&val));
    }
}

#[test]
fn thread_rng_float_bounds() {
    for _ in 0..100 {
        let val = ThreadRng::float();
        assert!((0.0..1.0).contains(&val));
    }
}

#[test]
fn thread_rng_pick_from_slice() {
    let items = [10, 20, 30, 40, 50];
    for _ in 0..50 {
        let picked = ThreadRng::pick(&items).unwrap();
        assert!(items.contains(picked));
    }
}

#[test]
fn thread_rng_pick_empty() {
    let empty: [i32; 0] = [];
    assert!(ThreadRng::pick(&empty).is_none());
}

#[test]
fn thread_rng_picks_count() {
    let items = [1, 2, 3, 4, 5];
    let picked = ThreadRng::picks(&items, 3);
    assert_eq!(picked.len(), 3);
}

#[test]
fn thread_rng_picks_capped_at_len() {
    let items = [1, 2, 3];
    let picked = ThreadRng::picks(&items, 100);
    assert_eq!(picked.len(), 3);
}

#[test]
fn thread_rng_picks_zero() {
    let items = [1, 2, 3];
    let picked = ThreadRng::picks(&items, 0);
    assert!(picked.is_empty());
}

#[test]
fn thread_rng_picks_empty_slice() {
    let empty: [i32; 0] = [];
    let picked = ThreadRng::picks(&empty, 5);
    assert!(picked.is_empty());
}

#[test]
fn thread_rng_shuffle_preserves_elements() {
    let mut items = vec![1, 2, 3, 4, 5];
    let original = items.clone();
    ThreadRng::shuffle(&mut items);
    items.sort();
    let mut sorted_original = original;
    sorted_original.sort();
    assert_eq!(items, sorted_original);
}

#[test]
fn thread_rng_shuffle_empty() {
    let mut empty: Vec<i32> = vec![];
    ThreadRng::shuffle(&mut empty);
    assert!(empty.is_empty());
}

#[test]
fn thread_rng_shuffle_single() {
    let mut single = vec![42];
    ThreadRng::shuffle(&mut single);
    assert_eq!(single, vec![42]);
}

// ── Statistical distribution tests ──────────────────────────────

#[test]
fn random_bool_roughly_fair() {
    let count = 10_000;
    let trues: usize = (0..count).filter(|_| random_bool()).count();
    // With 10k trials, expect ~5000 trues. Allow wide margin (40-60%).
    assert!(
        trues > 3500 && trues < 6500,
        "Got {} trues out of {}",
        trues,
        count
    );
}

#[test]
fn random_in_range_covers_all_values() {
    let mut seen = [false; 10];
    for _ in 0..1000 {
        let val = random_in_range(0, 9) as usize;
        seen[val] = true;
    }
    assert!(seen.iter().all(|&s| s), "All values 0-9 should be seen");
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_random_string_correct_length(len in 0usize..100) {
        let s = random_string(len);
        prop_assert_eq!(s.len(), len);
    }

    #[test]
    fn prop_random_lowercase_string_all_lowercase(len in 1usize..50) {
        let s = random_lowercase_string(len);
        prop_assert!(s.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn prop_random_uppercase_string_all_uppercase(len in 1usize..50) {
        let s = random_uppercase_string(len);
        prop_assert!(s.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn prop_random_bytes_correct_length(len in 0usize..200) {
        let bytes = random_bytes(len);
        prop_assert_eq!(bytes.len(), len);
    }

    #[test]
    fn prop_random_in_range_within_bounds(min in -100i32..0, max in 0i32..100) {
        let val = random_in_range(min, max);
        prop_assert!((min..=max).contains(&val));
    }

    #[test]
    fn prop_shuffle_preserves_length(len in 0usize..20) {
        let mut items: Vec<usize> = (0..len).collect();
        ThreadRng::shuffle(&mut items);
        prop_assert_eq!(items.len(), len);
    }
}
