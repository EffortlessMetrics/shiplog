//! Base64 encoding/decoding utilities for shiplog.
//!
//! This crate provides Base64 encoding/decoding utilities for the shiplog ecosystem.

use base64::{Engine as _, engine::general_purpose::STANDARD};

/// Encodes bytes to base64 string
pub fn encode(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// Decodes a base64 string to bytes
pub fn decode(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    STANDARD.decode(encoded)
}

/// Encodes bytes to base64 URL-safe string
pub fn encode_url_safe(data: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
}

/// Decodes a base64 URL-safe string to bytes
pub fn decode_url_safe(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded)
}

/// Encodes a string to base64 (UTF-8 bytes)
pub fn encode_string(data: &str) -> String {
    encode(data.as_bytes())
}

/// Decodes a base64 string to a UTF-8 string
pub fn decode_to_string(encoded: &str) -> Result<String, base64::DecodeError> {
    let bytes = decode(encoded)?;
    String::from_utf8(bytes).map_err(|_| base64::DecodeError::InvalidByte(0, 0))
}

/// Validates if a string is valid base64
pub fn is_valid_base64(encoded: &str) -> bool {
    decode(encoded).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let data = b"Hello, World!";
        let encoded = encode(data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
        
        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_encode_url_safe() {
        let data = b"test+/data";
        let encoded = encode_url_safe(data);
        assert_eq!(encoded, "dGVzdCsvZGF0YQ");
    }

    #[test]
    fn test_decode_url_safe() {
        let encoded = "dGVzdCsvZGF0YQ";
        let decoded = decode_url_safe(encoded).unwrap();
        assert_eq!(decoded, b"test+/data");
    }

    #[test]
    fn test_encode_string() {
        let encoded = encode_string("Hello");
        assert_eq!(encoded, "SGVsbG8=");
    }

    #[test]
    fn test_decode_to_string() {
        let encoded = "SGVsbG8=";
        let decoded = decode_to_string(encoded).unwrap();
        assert_eq!(decoded, "Hello");
    }

    #[test]
    fn test_is_valid_base64() {
        assert!(is_valid_base64("SGVsbG8="));
        assert!(!is_valid_base64("not-valid!!!"));
    }

    #[test]
    fn test_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog";
        let encoded = encode(original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(original.as_slice(), decoded);
    }
}
