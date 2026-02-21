//! Random number generation utilities for shiplog.
//!
//! Provides convenient random number generation utilities using the
//! standard library's RNG.

use rand::Rng;
use rand::rng;
use rand::seq::SliceRandom;
use rand::prelude::IndexedRandom;

/// Generate a random integer in the given range.
pub fn random_in_range(min: i32, max: i32) -> i32 {
    rng().random_range(min..=max)
}

/// Generate a random floating-point number between 0 and 1.
pub fn random_float() -> f64 {
    rng().random()
}

/// Generate a random boolean.
pub fn random_bool() -> bool {
    rng().random()
}

/// Generate a random byte.
pub fn random_byte() -> u8 {
    rng().random()
}

/// Generate a random character from the English alphabet (uppercase).
pub fn random_uppercase_char() -> char {
    let idx = rng().random_range(0..26);
    (b'A' + idx) as char
}

/// Generate a random character from the English alphabet (lowercase).
pub fn random_lowercase_char() -> char {
    let idx = rng().random_range(0..26);
    (b'a' + idx) as char
}

/// Generate a random alphanumeric character.
pub fn random_alphanumeric_char() -> char {
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    let idx = rng().random_range(0..chars.len());
    chars[idx]
}

/// Generate a random string of the given length.
pub fn random_string(length: usize) -> String {
    let chars: Vec<char> = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789".chars().collect();
    (0..length).map(|_| chars[rng().random_range(0..chars.len())]).collect()
}

/// Generate a random string of lowercase letters.
pub fn random_lowercase_string(length: usize) -> String {
    (0..length).map(|_| {
        let idx = rng().random_range(0..26);
        (b'a' + idx) as char
    }).collect()
}

/// Generate a random string of uppercase letters.
pub fn random_uppercase_string(length: usize) -> String {
    (0..length).map(|_| {
        let idx = rng().random_range(0..26);
        (b'A' + idx) as char
    }).collect()
}

/// Generate random bytes of the given length.
pub fn random_bytes(length: usize) -> Vec<u8> {
    (0..length).map(|_| rng().random()).collect()
}

/// A thread-local random number generator.
pub struct ThreadRng;

impl ThreadRng {
    /// Generate a random integer in the given range.
    pub fn range(min: i32, max: i32) -> i32 {
        rng().random_range(min..=max)
    }

    /// Generate a random floating-point number between 0 and 1.
    pub fn float() -> f64 {
        rng().random()
    }

    /// Generate a random boolean.
    pub fn bool() -> bool {
        rng().random()
    }

    /// Shuffle a slice randomly.
    pub fn shuffle<T: Clone>(slice: &mut [T]) {
        slice.shuffle(&mut rng());
    }

    /// Pick a random element from a slice.
    pub fn pick<T>(slice: &[T]) -> Option<&T> {
        if slice.is_empty() {
            None
        } else {
            slice.choose(&mut rng())
        }
    }

    /// Pick multiple random elements from a slice without replacement.
    pub fn picks<T>(slice: &[T], count: usize) -> Vec<&T> {
        if slice.is_empty() || count == 0 {
            return vec![];
        }
        let count = count.min(slice.len());
        slice.choose_multiple(&mut rng(), count).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_in_range_test() {
        for _ in 0..100 {
            let val = random_in_range(1, 10);
            assert!(val >= 1 && val <= 10);
        }
    }

    #[test]
    fn random_float_test() {
        for _ in 0..100 {
            let val = random_float();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn random_bool_test() {
        let mut has_true = false;
        let mut has_false = false;
        for _ in 0..100 {
            if random_bool() {
                has_true = true;
            } else {
                has_false = true;
            }
        }
        assert!(has_true && has_false);
    }

    #[test]
    fn random_byte_test() {
        for _ in 0..100 {
            let val = random_byte();
            assert!(val <= 255);
        }
    }

    #[test]
    fn random_uppercase_char_test() {
        for _ in 0..100 {
            let c = random_uppercase_char();
            assert!(c.is_ascii_uppercase());
        }
    }

    #[test]
    fn random_lowercase_char_test() {
        for _ in 0..100 {
            let c = random_lowercase_char();
            assert!(c.is_ascii_lowercase());
        }
    }

    #[test]
    fn random_alphanumeric_char_test() {
        for _ in 0..100 {
            let c = random_alphanumeric_char();
            assert!(c.is_ascii_alphanumeric());
        }
    }

    #[test]
    fn random_string_test() {
        let s = random_string(10);
        assert_eq!(s.len(), 10);
        assert!(s.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn random_lowercase_string_test() {
        let s = random_lowercase_string(10);
        assert_eq!(s.len(), 10);
        assert!(s.chars().all(|c| c.is_ascii_lowercase()));
    }

    #[test]
    fn random_uppercase_string_test() {
        let s = random_uppercase_string(10);
        assert_eq!(s.len(), 10);
        assert!(s.chars().all(|c| c.is_ascii_uppercase()));
    }

    #[test]
    fn random_bytes_test() {
        let bytes = random_bytes(16);
        assert_eq!(bytes.len(), 16);
    }

    #[test]
    fn thread_rng_range_test() {
        for _ in 0..100 {
            let val = ThreadRng::range(1, 10);
            assert!(val >= 1 && val <= 10);
        }
    }

    #[test]
    fn thread_rng_float_test() {
        for _ in 0..100 {
            let val = ThreadRng::float();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn thread_rng_bool_test() {
        let mut has_true = false;
        let mut has_false = false;
        for _ in 0..100 {
            if ThreadRng::bool() {
                has_true = true;
            } else {
                has_false = true;
            }
        }
        assert!(has_true && has_false);
    }

    #[test]
    fn thread_rng_pick_test() {
        let slice = [1, 2, 3, 4, 5];
        let picked = ThreadRng::pick(&slice);
        assert!(picked.is_some());
    }

    #[test]
    fn thread_rng_pick_empty_test() {
        let slice: [i32; 0] = [];
        let picked = ThreadRng::pick(&slice);
        assert!(picked.is_none());
    }

    #[test]
    fn thread_rng_picks_test() {
        let slice = [1, 2, 3, 4, 5];
        let picks = ThreadRng::picks(&slice, 3);
        assert_eq!(picks.len(), 3);
    }

    #[test]
    fn thread_rng_picks_count_greater_than_len_test() {
        let slice = [1, 2, 3];
        let picks = ThreadRng::picks(&slice, 10);
        assert_eq!(picks.len(), 3);
    }

    #[test]
    fn thread_rng_shuffle_test() {
        let mut original = vec![1, 2, 3, 4, 5];
        let original_clone = original.clone();
        ThreadRng::shuffle(&mut original);
        // Very unlikely to remain the same
        assert!(original != original_clone || original.len() <= 1);
    }
}
