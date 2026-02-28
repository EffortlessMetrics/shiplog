//! Integration tests for shiplog-crypto.

use proptest::prelude::*;
use shiplog_crypto::*;

// ── SHA-256 known-answer tests ──────────────────────────────────

#[test]
fn sha256_hello_world() {
    let hash = Hash::sha256(b"hello world");
    assert_eq!(
        hash.value,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
    assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
}

#[test]
fn sha256_empty() {
    let hash = Hash::sha256(b"");
    assert_eq!(
        hash.value,
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

// ── SHA-512 known-answer tests ──────────────────────────────────

#[test]
fn sha512_hello_world() {
    let hash = Hash::sha512(b"hello world");
    assert_eq!(hash.algorithm, HashAlgorithm::Sha512);
    assert_eq!(hash.value.len(), 128); // SHA-512 = 128 hex chars
}

#[test]
fn sha512_empty() {
    let hash = Hash::sha512(b"");
    assert_eq!(
        hash.value,
        "cf83e1357eefb8bdf1542850d66d8007d620e4050b5715dc83f4a921d36ce9ce47d0d13c5d85f2b0ff8318d2877eec2f63b931bd47417a81a538327af927da3e"
    );
}

// ── Hash::compute tests ─────────────────────────────────────────

#[test]
fn compute_sha256() {
    let h = Hash::compute(b"test", HashAlgorithm::Sha256);
    assert_eq!(h.algorithm, HashAlgorithm::Sha256);
    assert_eq!(h.value, Hash::sha256(b"test").value);
}

#[test]
fn compute_sha512() {
    let h = Hash::compute(b"test", HashAlgorithm::Sha512);
    assert_eq!(h.algorithm, HashAlgorithm::Sha512);
    assert_eq!(h.value, Hash::sha512(b"test").value);
}

// ── Hash::verify tests ──────────────────────────────────────────

#[test]
fn verify_correct_data() {
    let hash = Hash::sha256(b"secret message");
    assert!(hash.verify(b"secret message"));
}

#[test]
fn verify_incorrect_data() {
    let hash = Hash::sha256(b"secret message");
    assert!(!hash.verify(b"wrong message"));
}

#[test]
fn verify_sha512() {
    let hash = Hash::sha512(b"data");
    assert!(hash.verify(b"data"));
    assert!(!hash.verify(b"other"));
}

// ── XOR cipher tests ────────────────────────────────────────────

#[test]
fn xor_cipher_encrypt_decrypt_roundtrip() {
    let cipher = XorCipher::new(b"secret_key");
    let plaintext = b"Hello, World!";
    let encrypted = cipher.encrypt(plaintext);
    let decrypted = cipher.decrypt(&encrypted);
    assert_eq!(plaintext.to_vec(), decrypted);
}

#[test]
fn xor_cipher_encrypt_changes_data() {
    let cipher = XorCipher::new(b"key");
    let plaintext = b"hello";
    let encrypted = cipher.encrypt(plaintext);
    assert_ne!(plaintext.to_vec(), encrypted);
}

#[test]
fn xor_cipher_single_byte_key() {
    let cipher = XorCipher::new(vec![0xAA]);
    let plaintext = b"test";
    let encrypted = cipher.encrypt(plaintext);
    let decrypted = cipher.decrypt(&encrypted);
    assert_eq!(plaintext.to_vec(), decrypted);
}

#[test]
fn xor_cipher_empty_data() {
    let cipher = XorCipher::new(b"key");
    let encrypted = cipher.encrypt(b"");
    assert!(encrypted.is_empty());
    let decrypted = cipher.decrypt(b"");
    assert!(decrypted.is_empty());
}

#[test]
fn xor_cipher_symmetric() {
    let cipher = XorCipher::new(b"key");
    let data = b"some data";
    let encrypted = cipher.encrypt(data);
    // XOR is symmetric: encrypt(encrypt(x)) = x
    let double_encrypted = cipher.encrypt(&encrypted);
    assert_eq!(data.to_vec(), double_encrypted);
}

// ── hash_string / verify_hash tests ─────────────────────────────

#[test]
fn hash_string_length() {
    let hash = hash_string("test");
    assert_eq!(hash.len(), 64); // SHA-256 hex
}

#[test]
fn hash_string_deterministic() {
    let h1 = hash_string("hello");
    let h2 = hash_string("hello");
    assert_eq!(h1, h2);
}

#[test]
fn verify_hash_correct() {
    let hash = hash_string("test input");
    assert!(verify_hash("test input", &hash));
}

#[test]
fn verify_hash_incorrect() {
    let hash = hash_string("test input");
    assert!(!verify_hash("wrong input", &hash));
}

#[test]
fn verify_hash_empty_string() {
    let hash = hash_string("");
    assert!(verify_hash("", &hash));
}

// ── HashAlgorithm tests ─────────────────────────────────────────

#[test]
fn hash_algorithm_default_is_sha256() {
    assert_eq!(HashAlgorithm::default(), HashAlgorithm::Sha256);
}

// ── Serde tests ─────────────────────────────────────────────────

#[test]
fn hash_algorithm_serde() {
    let json = serde_json::to_string(&HashAlgorithm::Sha256).unwrap();
    assert_eq!(json, "\"sha256\"");
    let json512 = serde_json::to_string(&HashAlgorithm::Sha512).unwrap();
    assert_eq!(json512, "\"sha512\"");
}

#[test]
fn hash_serde_roundtrip() {
    let hash = Hash::sha256(b"test");
    let json = serde_json::to_string(&hash).unwrap();
    let deserialized: Hash = serde_json::from_str(&json).unwrap();
    assert_eq!(hash.algorithm, deserialized.algorithm);
    assert_eq!(hash.value, deserialized.value);
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_hash_algorithms() {
    let inputs = [b"hello" as &[u8], b"world", b"", b"shiplog"];
    let formatted: Vec<String> = inputs
        .iter()
        .map(|data| {
            let sha256 = Hash::sha256(data);
            let sha512 = Hash::sha512(data);
            format!(
                "{:>10} sha256={} sha512_prefix={}",
                String::from_utf8_lossy(data),
                sha256.value,
                &sha512.value[..16]
            )
        })
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

// ── Edge cases ──────────────────────────────────────────────────

#[test]
fn hash_large_data() {
    let large = vec![0u8; 1_000_000]; // 1MB
    let hash = Hash::sha256(&large);
    assert_eq!(hash.value.len(), 64);
}

#[test]
fn hash_binary_data() {
    let data: Vec<u8> = (0..=255).collect();
    let hash = Hash::sha256(&data);
    assert!(hash.verify(&data));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_sha256_deterministic(data in proptest::collection::vec(any::<u8>(), 0..200)) {
        let h1 = Hash::sha256(&data);
        let h2 = Hash::sha256(&data);
        prop_assert_eq!(h1.value, h2.value);
    }

    #[test]
    fn prop_sha256_length_always_64(data in proptest::collection::vec(any::<u8>(), 0..200)) {
        let hash = Hash::sha256(&data);
        prop_assert_eq!(hash.value.len(), 64);
    }

    #[test]
    fn prop_sha512_length_always_128(data in proptest::collection::vec(any::<u8>(), 0..200)) {
        let hash = Hash::sha512(&data);
        prop_assert_eq!(hash.value.len(), 128);
    }

    #[test]
    fn prop_hash_verify_roundtrip(data in proptest::collection::vec(any::<u8>(), 0..200)) {
        let hash = Hash::sha256(&data);
        prop_assert!(hash.verify(&data));
    }

    #[test]
    fn prop_xor_cipher_roundtrip(
        key in proptest::collection::vec(any::<u8>(), 1..20),
        data in proptest::collection::vec(any::<u8>(), 0..200),
    ) {
        let cipher = XorCipher::new(key);
        let encrypted = cipher.encrypt(&data);
        let decrypted = cipher.decrypt(&encrypted);
        prop_assert_eq!(data, decrypted);
    }

    #[test]
    fn prop_xor_cipher_symmetric(
        key in proptest::collection::vec(any::<u8>(), 1..20),
        data in proptest::collection::vec(any::<u8>(), 0..100),
    ) {
        let cipher = XorCipher::new(key);
        let double = cipher.encrypt(&cipher.encrypt(&data));
        prop_assert_eq!(data, double);
    }

    #[test]
    fn prop_hash_string_deterministic(s in "\\PC{0,100}") {
        let h1 = hash_string(&s);
        let h2 = hash_string(&s);
        prop_assert_eq!(h1, h2);
    }

    #[test]
    fn prop_verify_hash_roundtrip(s in "[a-zA-Z0-9]{0,50}") {
        let hash = hash_string(&s);
        prop_assert!(verify_hash(&s, &hash));
    }
}
