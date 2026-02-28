//! Integration tests for shiplog-hash.

use proptest::prelude::*;
use shiplog_hash::*;

// ── SHA-256 known-answer tests ──────────────────────────────────

#[test]
fn sha256_empty_string() {
    let hash = Hash::hash_str("");
    assert_eq!(
        hash.0,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn sha256_hello() {
    let hash = Hash::hash_str("hello");
    assert_eq!(
        hash.0,
        "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    );
}

#[test]
fn sha256_hello_world() {
    let hash = Hash::hash_str("hello world");
    assert_eq!(
        hash.0,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn sha256_from_bytes_matches_from_str() {
    let h1 = Hash::hash_str("test");
    let h2 = Hash::from_bytes(b"test");
    assert_eq!(h1, h2);
}

// ── Hash methods ────────────────────────────────────────────────

#[test]
fn hash_verify_correct() {
    let hash = Hash::hash_str("test data");
    assert!(hash.verify("test data"));
}

#[test]
fn hash_verify_incorrect() {
    let hash = Hash::hash_str("test data");
    assert!(!hash.verify("other data"));
}

#[test]
fn hash_prefix() {
    let hash = Hash::hash_str("hello");
    let prefix = hash.prefix(8);
    assert_eq!(prefix.len(), 8);
    assert_eq!(prefix, "2cf24dba");
}

#[test]
fn hash_prefix_longer_than_hash() {
    let hash = Hash::hash_str("hello");
    let prefix = hash.prefix(100);
    assert_eq!(prefix.len(), 64); // SHA-256 is 64 hex chars
}

#[test]
fn hash_prefix_zero() {
    let hash = Hash::hash_str("hello");
    let prefix = hash.prefix(0);
    assert!(prefix.is_empty());
}

#[test]
fn hash_as_bytes() {
    let hash = Hash::hash_str("hello");
    let bytes = hash.as_bytes();
    assert_eq!(bytes.len(), 32); // SHA-256 = 32 bytes
}

#[test]
fn hash_display() {
    let hash = Hash::hash_str("hello");
    let display = format!("{}", hash);
    assert_eq!(display, hash.0);
}

#[test]
fn hash_default_is_empty_string_hash() {
    let hash = Hash::default();
    let empty_hash = Hash::hash_str("");
    assert_eq!(hash, empty_hash);
}

// ── hash_items tests ────────────────────────────────────────────

#[test]
fn hash_items_deterministic() {
    let h1 = hash_items(&["a", "b", "c"]);
    let h2 = hash_items(&["a", "b", "c"]);
    assert_eq!(h1, h2);
}

#[test]
fn hash_items_order_matters() {
    let h1 = hash_items(&["a", "b"]);
    let h2 = hash_items(&["b", "a"]);
    assert_ne!(h1, h2);
}

#[test]
fn hash_items_single() {
    let h1 = hash_items(&["hello"]);
    let h2 = Hash::hash_str("hello");
    assert_eq!(h1, h2);
}

#[test]
fn hash_items_empty() {
    let h = hash_items(&[]);
    assert_eq!(h, Hash::hash_str(""));
}

// ── hash_content / hash_many ────────────────────────────────────

#[test]
fn hash_content_same_as_hash_str() {
    let h1 = hash_content("hello world");
    let h2 = Hash::hash_str("hello world");
    assert_eq!(h1, h2);
}

#[test]
fn hash_many_same_as_items() {
    let h1 = hash_many(&["a", "b"]);
    let h2 = hash_items(&["a", "b"]);
    assert_eq!(h1, h2);
}

// ── ContentHasher tests ─────────────────────────────────────────

#[test]
fn content_hasher_concatenation() {
    let mut hasher = ContentHasher::new();
    hasher.update("hello");
    hasher.update("world");
    let h = hasher.finalize();
    assert_eq!(h, Hash::hash_str("helloworld"));
}

#[test]
fn content_hasher_with_separator() {
    let mut hasher = ContentHasher::new();
    hasher.update_with_sep("hello", "|");
    hasher.update("world");
    let h = hasher.finalize();
    assert_eq!(h, Hash::hash_str("hello|world"));
}

#[test]
fn content_hasher_empty() {
    let hasher = ContentHasher::new();
    let h = hasher.finalize();
    assert_eq!(h, Hash::hash_str(""));
}

#[test]
fn content_hasher_default() {
    let hasher = ContentHasher::default();
    let h = hasher.finalize();
    assert_eq!(h, Hash::hash_str(""));
}

// ── Checksum tests ──────────────────────────────────────────────

#[test]
fn checksum_sha256_verify() {
    let cs = Checksum::sha256("hello");
    assert_eq!(cs.algorithm, "sha256");
    assert!(cs.verify("hello"));
    assert!(!cs.verify("world"));
}

#[test]
fn checksum_display() {
    let cs = Checksum::sha256("hello");
    let display = format!("{}", cs);
    assert!(display.starts_with("sha256:"));
    assert!(display.len() > 70); // "sha256:" + 64 hex chars
}

#[test]
fn checksum_unknown_algorithm_verify() {
    let cs = Checksum::new("md5", "deadbeef");
    assert!(!cs.verify("anything")); // Unknown algorithms always fail
}

// ── Serde tests ─────────────────────────────────────────────────

#[test]
fn hash_serde_roundtrip() {
    let hash = Hash::hash_str("test");
    let json = serde_json::to_string(&hash).unwrap();
    let deserialized: Hash = serde_json::from_str(&json).unwrap();
    assert_eq!(hash, deserialized);
}

#[test]
fn checksum_serde_roundtrip() {
    let cs = Checksum::sha256("test");
    let json = serde_json::to_string(&cs).unwrap();
    let deserialized: Checksum = serde_json::from_str(&json).unwrap();
    assert_eq!(cs, deserialized);
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_known_hashes() {
    let inputs = ["", "hello", "hello world", "shiplog", "test"];
    let formatted: Vec<String> = inputs
        .iter()
        .map(|&s| format!("{:>15} => {}", format!("\"{}\"", s), Hash::hash_str(s)))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_hash_deterministic(s in "\\PC{0,100}") {
        let h1 = Hash::hash_str(&s);
        let h2 = Hash::hash_str(&s);
        prop_assert_eq!(h1, h2);
    }

    #[test]
    fn prop_hash_length_always_64(s in "\\PC{0,100}") {
        let hash = Hash::hash_str(&s);
        prop_assert_eq!(hash.0.len(), 64);
    }

    #[test]
    fn prop_hash_all_hex_chars(s in "\\PC{0,50}") {
        let hash = Hash::hash_str(&s);
        prop_assert!(hash.0.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn prop_hash_verify_roundtrip(s in "\\PC{0,100}") {
        let hash = Hash::hash_str(&s);
        prop_assert!(hash.verify(&s));
    }

    #[test]
    fn prop_hash_bytes_roundtrip(s in "\\PC{0,50}") {
        let hash = Hash::hash_str(&s);
        let bytes = hash.as_bytes();
        prop_assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn prop_content_hasher_matches_direct(s in "\\PC{0,50}") {
        let mut hasher = ContentHasher::new();
        hasher.update(&s);
        let h = hasher.finalize();
        prop_assert_eq!(h, Hash::hash_str(&s));
    }

    #[test]
    fn prop_checksum_verify_roundtrip(s in "[a-zA-Z0-9]{1,50}") {
        let cs = Checksum::sha256(&s);
        prop_assert!(cs.verify(&s));
    }

    #[test]
    fn prop_hash_items_deterministic(
        items in proptest::collection::vec("[a-z]{1,10}", 1..5)
    ) {
        let refs: Vec<&str> = items.iter().map(|s| s.as_str()).collect();
        let h1 = hash_items(&refs);
        let h2 = hash_items(&refs);
        prop_assert_eq!(h1, h2);
    }
}
