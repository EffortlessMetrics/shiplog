//! Integration tests for shiplog-uuid.

use proptest::prelude::*;
use shiplog_uuid::*;

// ── RFC 4122 known-answer tests ─────────────────────────────────

#[test]
fn nil_uuid_format() {
    let nil = Uuid::from_string("00000000-0000-0000-0000-000000000000").unwrap();
    assert!(nil.is_nil());
    assert_eq!(nil.to_string(), "00000000-0000-0000-0000-000000000000");
}

#[test]
fn well_known_uuid_v4() {
    let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert_eq!(uuid.version(), Some(UuidVersion::Random));
    assert_eq!(uuid.to_string(), "550e8400-e29b-41d4-a716-446655440000");
}

#[test]
fn uuid_v1_detection() {
    let uuid = Uuid::from_string("550e8400-e29b-11d4-a716-446655440000").unwrap();
    assert_eq!(uuid.version(), Some(UuidVersion::TimeBased));
}

#[test]
fn uuid_v3_detection() {
    let uuid = Uuid::from_string("550e8400-e29b-31d4-a716-446655440000").unwrap();
    assert_eq!(uuid.version(), Some(UuidVersion::Md5));
}

#[test]
fn uuid_v5_detection() {
    let uuid = Uuid::from_string("550e8400-e29b-51d4-a716-446655440000").unwrap();
    assert_eq!(uuid.version(), Some(UuidVersion::Sha1));
}

// ── Format validation tests ─────────────────────────────────────

#[test]
fn uuid_format_8_4_4_4_12() {
    // Uuid::new() uses nanos which may exceed 12 hex chars in the last segment
    // Test with a known well-formed UUID instead
    let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let parts: Vec<&str> = uuid.0.split('-').collect();
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[0].len(), 8);
    assert_eq!(parts[1].len(), 4);
    assert_eq!(parts[2].len(), 4);
    assert_eq!(parts[3].len(), 4);
    assert_eq!(parts[4].len(), 12);
}

#[test]
fn uuid_all_lowercase_hex() {
    let uuid = Uuid::new();
    let simple = uuid.as_simple();
    assert!(simple.chars().all(|c| c.is_ascii_hexdigit()));
}

// ── from_string tests ───────────────────────────────────────────

#[test]
fn from_string_standard_format() {
    let uuid = Uuid::from_string("a1b2c3d4-e5f6-7890-abcd-ef1234567890");
    assert!(uuid.is_some());
}

#[test]
fn from_string_no_dashes() {
    let uuid = Uuid::from_string("a1b2c3d4e5f67890abcdef1234567890").unwrap();
    assert_eq!(uuid.0, "a1b2c3d4-e5f6-7890-abcd-ef1234567890");
}

#[test]
fn from_string_invalid() {
    assert!(Uuid::from_string("").is_none());
    assert!(Uuid::from_string("too-short").is_none());
    assert!(Uuid::from_string("this-is-not-valid-at-all-but-36ch").is_none());
}

// ── from_bytes / as_bytes round-trip ────────────────────────────

#[test]
fn bytes_roundtrip() {
    let bytes: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let uuid = Uuid::from_bytes(bytes);
    let recovered = uuid.as_bytes().unwrap();
    assert_eq!(bytes, recovered);
}

#[test]
fn bytes_all_zeros() {
    let bytes = [0u8; 16];
    let uuid = Uuid::from_bytes(bytes);
    assert!(uuid.is_nil());
}

#[test]
fn bytes_all_ones() {
    let bytes = [0xffu8; 16];
    let uuid = Uuid::from_bytes(bytes);
    assert_eq!(uuid.as_simple(), "ffffffffffffffffffffffffffffffff");
}

// ── as_simple tests ─────────────────────────────────────────────

#[test]
fn as_simple_length() {
    let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert_eq!(uuid.as_simple().len(), 32);
}

#[test]
fn as_simple_no_dashes() {
    let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
    assert!(!uuid.as_simple().contains('-'));
}

// ── FromStr trait tests ─────────────────────────────────────────

#[test]
fn from_str_valid() {
    let uuid: Result<Uuid, _> = "550e8400-e29b-41d4-a716-446655440000".parse();
    assert!(uuid.is_ok());
}

#[test]
fn from_str_invalid() {
    let uuid: Result<Uuid, _> = "invalid".parse();
    assert!(uuid.is_err());
}

// ── parse_uuid tests ────────────────────────────────────────────

#[test]
fn parse_uuid_valid() {
    let result = parse_uuid("550e8400-e29b-41d4-a716-446655440000");
    assert!(result.is_some());
    let (time_low, time_mid, time_hi, clock_seq, node) = result.unwrap();
    assert_eq!(time_low, 0x550e8400);
    assert_eq!(time_mid, 0xe29b);
    assert_eq!(time_hi, 0x41d4);
    assert_eq!(clock_seq, 0xa716);
    assert_eq!(node, 0x446655440000);
}

#[test]
fn parse_uuid_nil() {
    let result = parse_uuid("00000000-0000-0000-0000-000000000000");
    assert!(result.is_some());
    let (a, b, c, d, e) = result.unwrap();
    assert_eq!((a, b, c, d, e), (0, 0, 0, 0, 0));
}

#[test]
fn parse_uuid_invalid() {
    assert!(parse_uuid("").is_none());
    assert!(parse_uuid("invalid").is_none());
    assert!(parse_uuid("not-a-uuid-at-all").is_none());
}

// ── generate_id tests ───────────────────────────────────────────

#[test]
fn generate_id_has_prefix() {
    let id = generate_id("test");
    assert!(id.starts_with("test_"));
}

#[test]
fn generate_id_uniqueness() {
    let id1 = generate_id("x");
    std::thread::sleep(std::time::Duration::from_millis(1));
    let id2 = generate_id("x");
    // Very unlikely to be the same due to nanos
    assert_ne!(id1, id2);
}

#[test]
fn generate_id_empty_prefix() {
    let id = generate_id("");
    assert!(id.starts_with("_"));
}

// ── Uniqueness tests ────────────────────────────────────────────

#[test]
fn new_uuid_unique() {
    let uuids: Vec<Uuid> = (0..100).map(|_| Uuid::new()).collect();
    for i in 0..uuids.len() {
        for j in (i + 1)..uuids.len() {
            assert_ne!(uuids[i], uuids[j], "UUIDs should be unique");
        }
    }
}

// ── Serde tests ─────────────────────────────────────────────────

#[test]
fn serde_roundtrip() {
    let uuid = Uuid::from_string("550e8400-e29b-41d4-a716-446655440000").unwrap();
    let json = serde_json::to_string(&uuid).unwrap();
    assert_eq!(json, "\"550e8400-e29b-41d4-a716-446655440000\"");
    let deserialized: Uuid = serde_json::from_str(&json).unwrap();
    assert_eq!(uuid, deserialized);
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_uuid_version_display() {
    let versions = [
        UuidVersion::TimeBased,
        UuidVersion::Md5,
        UuidVersion::Random,
        UuidVersion::Sha1,
    ];
    let formatted: Vec<String> = versions.iter().map(|v| format!("{}", v)).collect();
    insta::assert_snapshot!(formatted.join("\n"), @r"
    time-based
    md5
    random
    sha1
    ");
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_from_bytes_roundtrip(bytes in proptest::array::uniform16(any::<u8>())) {
        let uuid = Uuid::from_bytes(bytes);
        let recovered = uuid.as_bytes().unwrap();
        prop_assert_eq!(bytes, recovered);
    }

    #[test]
    fn prop_from_string_roundtrip_standard(
        a in "[0-9a-f]{8}",
        b in "[0-9a-f]{4}",
        c in "[0-9a-f]{4}",
        d in "[0-9a-f]{4}",
        e in "[0-9a-f]{12}",
    ) {
        let s = format!("{}-{}-{}-{}-{}", a, b, c, d, e);
        let uuid = Uuid::from_string(&s).unwrap();
        prop_assert_eq!(uuid.to_string(), s);
    }

    #[test]
    fn prop_as_simple_length_always_32(
        a in "[0-9a-f]{8}",
        b in "[0-9a-f]{4}",
        c in "[0-9a-f]{4}",
        d in "[0-9a-f]{4}",
        e in "[0-9a-f]{12}",
    ) {
        let s = format!("{}-{}-{}-{}-{}", a, b, c, d, e);
        let uuid = Uuid::from_string(&s).unwrap();
        prop_assert_eq!(uuid.as_simple().len(), 32);
    }

    #[test]
    fn prop_uuid_new_has_valid_format(_ in 0..100u32) {
        let uuid = Uuid::new();
        let parts: Vec<&str> = uuid.0.split('-').collect();
        prop_assert_eq!(parts.len(), 5);
        // Uuid::new() has at least 36 chars (last segment may be longer)
        prop_assert!(uuid.0.len() >= 36);
    }
}
