use shiplog_base64::*;

#[test]
fn encode_decode_roundtrip() {
    let data = b"Hello, World!";
    let encoded = encode(data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn encode_known_value() {
    assert_eq!(encode(b"Hello"), "SGVsbG8=");
    assert_eq!(encode(b""), "");
}

#[test]
fn decode_invalid_input() {
    assert!(decode("!!!invalid!!!").is_err());
}

#[test]
fn url_safe_roundtrip() {
    let data = b"test+/data==";
    let encoded = encode_url_safe(data);
    let decoded = decode_url_safe(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn encode_decode_string() {
    let text = "Hello, 世界!";
    let encoded = encode_string(text);
    let decoded = decode_to_string(&encoded).unwrap();
    assert_eq!(decoded, text);
}

#[test]
fn is_valid_base64_check() {
    assert!(is_valid_base64("SGVsbG8="));
    assert!(is_valid_base64(""));
    assert!(!is_valid_base64("not-valid!!!"));
}

#[test]
fn empty_input() {
    assert_eq!(encode(b""), "");
    assert_eq!(decode("").unwrap(), b"");
    assert_eq!(encode_url_safe(b""), "");
}

#[test]
fn binary_data_roundtrip() {
    let data: Vec<u8> = (0..=255).collect();
    let encoded = encode(&data);
    let decoded = decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}
