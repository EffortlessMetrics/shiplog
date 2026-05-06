//! Fuzz harness for bundle checksum calculation.
//!
//! Exercises SHA-256 hashing of arbitrary content (the same algorithm used by
//! `shiplog-bundle` for `FileChecksum` generation) and verifies invariants:
//! determinism, valid hex output, and `FileChecksum` serde round-tripping.

#![no_main]

use libfuzzer_sys::fuzz_target;
use sha2::{Digest, Sha256};
use shiplog_schema::bundle::FileChecksum;

fuzz_target!(|data: &[u8]| {
    // Compute SHA-256 of arbitrary content (mirrors shiplog-bundle::sha256_file)
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hex::encode(hasher.finalize());

    // Invariant: digest is always 64 hex characters
    assert_eq!(digest.len(), 64, "SHA-256 hex digest must be 64 chars");
    assert!(
        digest.chars().all(|c| c.is_ascii_hexdigit()),
        "digest must be valid hex"
    );

    // Invariant: deterministic — same input always produces same hash
    let mut hasher2 = Sha256::new();
    hasher2.update(data);
    let digest2 = hex::encode(hasher2.finalize());
    assert_eq!(digest, digest2, "SHA-256 must be deterministic");

    // Fuzz FileChecksum serde round-trip with computed digest
    let fc = FileChecksum {
        path: "fuzz/test.bin".into(),
        sha256: digest.clone(),
        bytes: data.len() as u64,
    };
    let json = serde_json::to_string(&fc).expect("FileChecksum serialization must not panic");
    let back: FileChecksum =
        serde_json::from_str(&json).expect("FileChecksum deserialization must not panic");
    assert_eq!(fc, back, "FileChecksum serde round-trip must be lossless");

    // Also fuzz FileChecksum deserialization from arbitrary input
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = serde_json::from_str::<FileChecksum>(input);
    }
});
