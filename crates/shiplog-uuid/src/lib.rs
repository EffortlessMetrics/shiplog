//! UUID generation and utilities for shiplog.
//!
//! Provides UUID types and generation utilities for creating unique
//! identifiers in the shiplog pipeline.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A UUID wrapper that provides additional utilities.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Uuid(pub String);

impl Uuid {
    /// Generate a new UUID (using timestamp-based generation).
    ///
    /// Note: This is not a true UUID v1 but uses timestamp to generate
    /// unique-ish identifiers for use in shiplog.
    pub fn new() -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let random_part = (nanos % 65536) as u64;
        Self(format!(
            "{:08x}-{:04x}-0000-0000-{:012x}",
            nanos as u32, random_part, nanos
        ))
    }

    /// Create a UUID from a string (validated format).
    pub fn from_string(s: impl Into<String>) -> Option<Self> {
        let s = s.into();
        // Basic UUID format validation (8-4-4-4-12)
        if s.len() == 36 && s.chars().filter(|c| *c == '-').count() == 4 {
            Some(Self(s))
        } else if s.len() == 32 {
            // Try to format as UUID
            Some(Self(format!(
                "{}-{}-{}-{}-{}",
                &s[0..8],
                &s[8..12],
                &s[12..16],
                &s[16..20],
                &s[20..32]
            )))
        } else {
            None
        }
    }

    /// Create a UUID from raw bytes.
    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0],
            bytes[1],
            bytes[2],
            bytes[3],
            bytes[4],
            bytes[5],
            bytes[6],
            bytes[7],
            bytes[8],
            bytes[9],
            bytes[10],
            bytes[11],
            bytes[12],
            bytes[13],
            bytes[14],
            bytes[15]
        ))
    }

    /// Get the UUID as a simple string (no dashes).
    pub fn as_simple(&self) -> String {
        self.0.replace('-', "")
    }

    /// Get the UUID as raw bytes.
    pub fn as_bytes(&self) -> Option<[u8; 16]> {
        let parts: Vec<&str> = self.0.split('-').collect();
        if parts.len() != 5 {
            return None;
        }

        let mut bytes = [0u8; 16];
        let hex_chars: Vec<char> = parts.join("").chars().collect();

        for (i, b) in bytes.iter_mut().enumerate() {
            if i * 2 + 1 < hex_chars.len() {
                let high = hex_chars[i * 2].to_digit(16)?;
                let low = hex_chars[i * 2 + 1].to_digit(16)?;
                *b = ((high as u8) << 4) | (low as u8);
            }
        }

        Some(bytes)
    }

    /// Check if this UUID is nil (all zeros).
    pub fn is_nil(&self) -> bool {
        self.0.replace('-', "").chars().all(|c| c == '0')
    }

    /// Get the version of UUID (if determinable).
    pub fn version(&self) -> Option<UuidVersion> {
        let parts: Vec<&str> = self.0.split('-').collect();
        if parts.len() != 5 {
            return None;
        }
        // The third group starts with the version digit
        let third = parts.get(2)?.chars().next()?;
        match third {
            '1' => Some(UuidVersion::TimeBased),
            '3' => Some(UuidVersion::Md5),
            '4' => Some(UuidVersion::Random),
            '5' => Some(UuidVersion::Sha1),
            _ => None,
        }
    }
}

impl Default for Uuid {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Uuid {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::from_string(s).ok_or_else(|| "Invalid UUID format".to_string())
    }
}

/// UUID version variants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UuidVersion {
    /// Time-based UUID (v1)
    TimeBased,
    /// MD5-based UUID (v3)
    Md5,
    /// Random UUID (v4)
    Random,
    /// SHA-1 based UUID (v5)
    Sha1,
}

impl fmt::Display for UuidVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UuidVersion::TimeBased => write!(f, "time-based"),
            UuidVersion::Md5 => write!(f, "md5"),
            UuidVersion::Random => write!(f, "random"),
            UuidVersion::Sha1 => write!(f, "sha1"),
        }
    }
}

/// Generate a simple timestamp-based ID.
pub fn generate_id(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{}_{:x}", prefix, nanos)
}

/// Parse a UUID string into its component parts.
pub fn parse_uuid(s: &str) -> Option<(u32, u16, u16, u16, u64)> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 5 {
        return None;
    }

    let time_low: u32 = u32::from_str_radix(parts[0], 16).ok()?;
    let time_mid: u16 = u16::from_str_radix(parts[1], 16).ok()?;
    let time_hi_and_version: u16 = u16::from_str_radix(parts[2], 16).ok()?;
    let clock_seq: u16 = u16::from_str_radix(parts[3], 16).ok()?;
    let node: u64 = u64::from_str_radix(parts[4], 16).ok()?;

    Some((time_low, time_mid, time_hi_and_version, clock_seq, node))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_new_generates() {
        let uuid = Uuid::new();
        assert!(!uuid.0.is_empty());
        // UUID should have valid format (8-4-4-4-12)
        assert_eq!(uuid.0.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn uuid_from_string_valid() {
        let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(uuid.0, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn uuid_from_string_no_dashes() {
        let uuid = Uuid::from_string("550e8400e29b41d4a716446655440000").unwrap();
        assert_eq!(uuid.0, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn uuid_from_string_invalid() {
        let uuid = Uuid::from_string("invalid");
        assert!(uuid.is_none());
    }

    #[test]
    fn uuid_as_simple() {
        let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(uuid.as_simple(), "550e8400e29b41d4a716446655440000");
    }

    #[test]
    fn uuid_is_nil() {
        let nil_uuid = Uuid::from_string("00000000-0000-0000-0000-000000000000").unwrap();
        assert!(nil_uuid.is_nil());

        let uuid = Uuid::new();
        assert!(!uuid.is_nil());
    }

    #[test]
    fn uuid_display() {
        let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        assert_eq!(format!("{}", uuid), "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn uuid_from_str_trait() {
        let uuid: Uuid = "550e8400-e29b-41d4-a716-446655440000".parse().unwrap();
        assert_eq!(uuid.0, "550e8400-e29b-41d4-a716-446655440000");
    }

    #[test]
    fn generate_id_with_prefix() {
        let id = generate_id("test");
        assert!(id.starts_with("test_"));
    }

    #[test]
    fn parse_uuid_valid() {
        let result = parse_uuid("550e8400-e29b-41d4-a716-446655440000");
        assert!(result.is_some());
        let (time_low, time_mid, _time_hi, _clock_seq, _node) = result.unwrap();
        assert_eq!(time_low, 0x550e8400);
        assert_eq!(time_mid, 0xe29b);
    }

    #[test]
    fn parse_uuid_invalid() {
        let result = parse_uuid("invalid");
        assert!(result.is_none());
    }

    #[test]
    fn uuid_version_detection() {
        let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
        // Version is in the 3rd segment, 4th character
        // This is "4" which means random
        assert_eq!(uuid.version(), Some(UuidVersion::Random));
    }
}
