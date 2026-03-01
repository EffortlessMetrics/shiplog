use shiplog_hex::*;

#[test]
fn encode_decode_roundtrip() {
    let data = b"Hello, World!";
    let encoded = encode(data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn encode_known_value() {
    assert_eq!(encode(b"\x00\xff"), "00ff");
    assert_eq!(encode(b""), "");
}

#[test]
fn encode_upper_case() {
    assert_eq!(encode_upper(b"\x0a\x0b\x0c"), "0A0B0C");
}

#[test]
fn decode_invalid_hex() {
    assert!(decode("zz").is_err());
    assert!(decode("0").is_err()); // odd length
}

#[test]
fn is_valid_hex_check() {
    assert!(is_valid_hex("00ff"));
    assert!(is_valid_hex("AABB"));
    assert!(is_valid_hex(""));
    assert!(!is_valid_hex("zz"));
    assert!(!is_valid_hex("0")); // odd length
}

#[test]
fn hex_to_nibble_all_values() {
    assert_eq!(hex_to_nibble('0'), Some(0));
    assert_eq!(hex_to_nibble('9'), Some(9));
    assert_eq!(hex_to_nibble('a'), Some(10));
    assert_eq!(hex_to_nibble('f'), Some(15));
    assert_eq!(hex_to_nibble('A'), Some(10));
    assert_eq!(hex_to_nibble('F'), Some(15));
    assert_eq!(hex_to_nibble('g'), None);
    assert_eq!(hex_to_nibble(' '), None);
}

#[test]
fn byte_to_hex_values() {
    assert_eq!(byte_to_hex(0), '0');
    assert_eq!(byte_to_hex(15), 'f');
}

#[test]
fn binary_data_roundtrip() {
    let data: Vec<u8> = (0..=255).collect();
    let encoded = encode(&data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}
