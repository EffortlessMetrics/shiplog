//! Cryptographic utilities for shiplog.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};

/// Hash algorithm selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    /// SHA-256 hash (default)
    Sha256,
    /// SHA-512 hash
    Sha512,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        HashAlgorithm::Sha256
    }
}

/// Computed hash with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    pub value: String,
}

impl Hash {
    /// Compute SHA-256 hash of data
    pub fn sha256(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self {
            algorithm: HashAlgorithm::Sha256,
            value: hex::encode(result),
        }
    }

    /// Compute SHA-512 hash of data
    pub fn sha512(data: &[u8]) -> Self {
        let mut hasher = Sha512::new();
        hasher.update(data);
        let result = hasher.finalize();
        Self {
            algorithm: HashAlgorithm::Sha512,
            value: hex::encode(result),
        }
    }

    /// Compute hash using specified algorithm
    pub fn compute(data: &[u8], algorithm: HashAlgorithm) -> Self {
        match algorithm {
            HashAlgorithm::Sha256 => Self::sha256(data),
            HashAlgorithm::Sha512 => Self::sha512(data),
        }
    }

    /// Verify that data matches this hash
    pub fn verify(&self, data: &[u8]) -> bool {
        let computed = Self::compute(data, self.algorithm);
        self.value == computed.value
    }
}

/// Simple XOR-based encryption for basic obfuscation
/// Note: This is NOT secure encryption - use for obfuscation only
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XorCipher {
    key: Vec<u8>,
}

impl XorCipher {
    /// Create a new XOR cipher with the given key
    pub fn new(key: impl Into<Vec<u8>>) -> Self {
        Self { key: key.into() }
    }

    /// Encrypt data using XOR cipher
    pub fn encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ self.key[i % self.key.len()])
            .collect()
    }

    /// Decrypt data using XOR cipher (same operation as encrypt)
    pub fn decrypt(&self, data: &[u8]) -> Vec<u8> {
        // XOR is symmetric, so decryption is the same as encryption
        self.encrypt(data)
    }
}

/// Hash a string and return hex-encoded result
pub fn hash_string(input: &str) -> String {
    Hash::sha256(input.as_bytes()).value
}

/// Verify a string matches a hash
pub fn verify_hash(input: &str, expected_hash: &str) -> bool {
    hash_string(input) == expected_hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_hash() {
        let data = b"hello world";
        let hash = Hash::sha256(data);
        
        assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
        // Known SHA-256 hash of "hello world"
        assert_eq!(hash.value, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn sha512_hash() {
        let data = b"hello world";
        let hash = Hash::sha512(data);
        
        assert_eq!(hash.algorithm, HashAlgorithm::Sha512);
    }

    #[test]
    fn hash_verify() {
        let data = b"test data";
        let hash = Hash::sha256(data);
        
        assert!(hash.verify(data));
        assert!(!hash.verify(b"other data"));
    }

    #[test]
    fn xor_cipher() {
        let cipher = XorCipher::new(b"secret_key");
        
        let plaintext = b"Hello, World!";
        let encrypted = cipher.encrypt(plaintext);
        let decrypted = cipher.decrypt(&encrypted);
        
        assert_eq!(plaintext.to_vec(), decrypted);
    }

    #[test]
    fn hash_string_function() {
        let hash = hash_string("test");
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn verify_hash_function() {
        let hash = hash_string("hello");
        assert!(verify_hash("hello", &hash));
        assert!(!verify_hash("world", &hash));
    }
}
