use shiplog_chars::*;

#[test]
fn vowels_and_consonants() {
    for c in ['a', 'e', 'i', 'o', 'u', 'A', 'E'] {
        assert!(is_vowel(c), "{c} should be a vowel");
    }
    for c in ['b', 'c', 'd', 'f', 'B', 'Z'] {
        assert!(is_consonant(c), "{c} should be a consonant");
    }
    assert!(!is_vowel('1'));
    assert!(!is_consonant('1'));
}

#[test]
fn digit_classification() {
    for c in '0'..='9' {
        assert!(is_digit(c));
    }
    assert!(!is_digit('a'));
}

#[test]
fn hex_digit_classification() {
    for c in "0123456789abcdefABCDEF".chars() {
        assert!(is_hex_digit(c), "{c} should be hex");
    }
    assert!(!is_hex_digit('g'));
    assert!(!is_hex_digit('G'));
}

#[test]
fn octal_digit_classification() {
    for c in '0'..='7' {
        assert!(is_octal_digit(c));
    }
    assert!(!is_octal_digit('8'));
    assert!(!is_octal_digit('9'));
}

#[test]
fn case_conversion() {
    assert_eq!(to_upper('a'), 'A');
    assert_eq!(to_lower('Z'), 'z');
    assert!(is_uppercase('A'));
    assert!(is_lowercase('z'));
}

#[test]
fn ascii_roundtrip() {
    for val in 32..=126u8 {
        let c = from_ascii(val);
        assert_eq!(ascii_value(c), val);
    }
}

#[test]
fn control_and_printable() {
    assert!(is_control('\0'));
    assert!(is_control('\x7f'));
    assert!(!is_control('a'));
    assert!(is_printable('a'));
    assert!(is_printable(' '));
    assert!(!is_printable('\n'));
}

#[test]
fn whitespace_and_punctuation() {
    assert!(is_whitespace(' '));
    assert!(is_whitespace('\t'));
    assert!(is_punctuation('!'));
    assert!(is_punctuation('.'));
    assert!(!is_punctuation('a'));
}

#[test]
fn alphanumeric_and_letter() {
    assert!(is_alphanumeric('a'));
    assert!(is_alphanumeric('5'));
    assert!(!is_alphanumeric('!'));
    assert!(is_letter('Z'));
    assert!(!is_letter('9'));
}

#[test]
fn is_ascii_check() {
    assert!(is_ascii('z'));
    assert!(!is_ascii('é'));
    assert!(!is_ascii('日'));
}
