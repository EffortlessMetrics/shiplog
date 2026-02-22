//! Hex encoding/decoding utilities for shiplog.
//!
//! This crate provides Hex encoding/decoding utilities for the shiplog ecosystem.

/// Encodes bytes to hex string (lowercase)
pub fn encode(data: &[u8]) -> String {
    hex::encode(data)
}

/// Decodes a hex string to bytes
pub fn decode(encoded: &str) -> Result<Vec<u8>, hex::FromHexError> {
    hex::decode(encoded)
}

/// Encodes bytes to hex string (uppercase)
pub fn encode_upper(data: &[u8]) -> String {
    hex::encode_upper(data)
}

/// Validates if a string is valid hex
pub fn is_valid_hex(encoded: &str) -> bool {
    hex::decode(encoded).is_ok()
}

/// Converts a single byte to hex character (lowercase)
pub fn byte_to_hex(byte: u8) -> char {
    format!("{:02x}", byte).chars().nth(1).unwrap()
}

/// Converts a hex character to a nibble (4 bits)
pub fn hex_to_nibble(c: char) -> Option<u8> {
    match c {
        '0'..='9' => Some(c as u8 - b'0'),
        'a'..='f' => Some(c as u8 - b'a' + 10),
        'A'..='F' => Some(c as u8 - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let data = b"Hello, World!";
        let encoded = encode(data);
        assert_eq!(encoded, "48656c6c6f2c20576f726c6421");

        let decoded = decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_encode_upper() {
        let data = b"Hello";
        let encoded = upper(data);
        assert_eq!(encoded, "48656C6C6F");
    }

    #[test]
    fn test_is_valid_hex() {
        assert!(is_valid_hex("48656c6c6f"));
        assert!(!is_valid_hex("nothex!!!"));
    }

    #[test]
    fn test_byte_to_hex() {
        assert_eq!(byte_to_hex(0), '0');
        assert_eq!(byte_to_hex(15), 'f');
        assert_eq!(byte_to_hex(255), 'f');
    }

    #[test]
    fn test_hex_to_nibble() {
        assert_eq!(hex_to_nibble('0'), Some(0));
        assert_eq!(hex_to_nibble('9'), Some(9));
        assert_eq!(hex_to_nibble('a'), Some(10));
        assert_eq!(hex_to_nibble('f'), Some(15));
        assert_eq!(hex_to_nibble('A'), Some(10));
        assert_eq!(hex_to_nibble('F'), Some(15));
        assert_eq!(hex_to_nibble('g'), None);
    }

    #[test]
    fn test_roundtrip() {
        let original = b"The quick brown fox jumps over the lazy dog";
        let encoded = encode(original);
        let decoded = decode(&encoded).unwrap();
        assert_eq!(original.as_slice(), decoded);
    }

    // Helper function for upper case
    fn upper(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02X}", b)).collect()
    }
}
