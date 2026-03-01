//! Integration tests for shiplog-validator.

use shiplog_validator::*;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_email_empty_error() {
    let err = EmailValidator::validate("").unwrap_err();
    insta::assert_yaml_snapshot!("email_empty_error", err);
}

#[test]
fn snapshot_email_invalid_format_error() {
    let err = EmailValidator::validate("not-an-email").unwrap_err();
    insta::assert_yaml_snapshot!("email_invalid_format_error", err);
}

#[test]
fn snapshot_url_empty_error() {
    let err = UrlValidator::validate("").unwrap_err();
    insta::assert_yaml_snapshot!("url_empty_error", err);
}

#[test]
fn snapshot_username_too_short_error() {
    let err = UsernameValidator::validate("ab").unwrap_err();
    insta::assert_yaml_snapshot!("username_too_short_error", err);
}

#[test]
fn snapshot_username_too_long_error() {
    let err = UsernameValidator::validate(&"a".repeat(33)).unwrap_err();
    insta::assert_yaml_snapshot!("username_too_long_error", err);
}

#[test]
fn snapshot_username_invalid_chars_error() {
    let err = UsernameValidator::validate("user@name!").unwrap_err();
    insta::assert_yaml_snapshot!("username_invalid_chars_error", err);
}

#[test]
fn snapshot_password_empty_error() {
    let err = PasswordValidator::validate("").unwrap_err();
    insta::assert_yaml_snapshot!("password_empty_error", err);
}

#[test]
fn snapshot_password_too_short_error() {
    let err = PasswordValidator::validate("Sh0rt").unwrap_err();
    insta::assert_yaml_snapshot!("password_too_short_error", err);
}

#[test]
fn snapshot_password_no_variety_error() {
    let err = PasswordValidator::validate("alllowercase1").unwrap_err();
    insta::assert_yaml_snapshot!("password_no_variety_error", err);
}

#[test]
fn snapshot_ipv4_wrong_octets_error() {
    let err = IpAddressValidator::validate_ipv4("1.2.3").unwrap_err();
    insta::assert_yaml_snapshot!("ipv4_wrong_octets_error", err);
}

#[test]
fn snapshot_ipv4_octet_out_of_range_error() {
    let err = IpAddressValidator::validate_ipv4("192.168.1.256").unwrap_err();
    insta::assert_yaml_snapshot!("ipv4_octet_out_of_range_error", err);
}

#[test]
fn snapshot_range_below_min_error() {
    let validator = RangeValidator::new(Some(10), Some(100));
    let err = validator.validate(5).unwrap_err();
    insta::assert_yaml_snapshot!("range_below_min_error", err);
}

#[test]
fn snapshot_range_above_max_error() {
    let validator = RangeValidator::new(Some(10), Some(100));
    let err = validator.validate(200).unwrap_err();
    insta::assert_yaml_snapshot!("range_above_max_error", err);
}

#[test]
fn snapshot_error_code_displays() {
    let codes = [
        ErrorCode::Empty,
        ErrorCode::InvalidFormat,
        ErrorCode::OutOfRange,
        ErrorCode::InvalidValue,
        ErrorCode::TooLong,
        ErrorCode::TooShort,
        ErrorCode::Missing,
    ];
    let displays: Vec<String> = codes.iter().map(|c| format!("{c}")).collect();
    insta::assert_yaml_snapshot!("all_error_code_displays", displays);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn valid_emails_pass(
            local in "[a-z]{1,10}",
            domain in "[a-z]{1,10}",
            tld in "[a-z]{2,4}"
        ) {
            let email = format!("{local}@{domain}.{tld}");
            prop_assert!(EmailValidator::validate(&email).is_ok());
        }

        #[test]
        fn valid_urls_pass(domain in "[a-z]{1,10}", tld in "[a-z]{2,4}") {
            let url = format!("https://{domain}.{tld}");
            prop_assert!(UrlValidator::validate(&url).is_ok());
        }

        #[test]
        fn valid_usernames_pass(name in "[a-zA-Z0-9_-]{3,32}") {
            prop_assert!(UsernameValidator::validate(&name).is_ok());
        }

        #[test]
        fn short_usernames_fail(name in "[a-z]{1,2}") {
            prop_assert!(UsernameValidator::validate(&name).is_err());
        }

        #[test]
        fn valid_ipv4_pass(
            a in 0_u8..=255,
            b in 0_u8..=255,
            c in 0_u8..=255,
            d in 0_u8..=255
        ) {
            let ip = format!("{a}.{b}.{c}.{d}");
            prop_assert!(IpAddressValidator::validate_ipv4(&ip).is_ok());
        }

        #[test]
        fn range_within_bounds_passes(val in 10_i32..=100) {
            let validator = RangeValidator::new(Some(10), Some(100));
            prop_assert!(validator.validate(val).is_ok());
        }

        #[test]
        fn range_below_min_fails(val in i32::MIN..10) {
            let validator = RangeValidator::new(Some(10), Some(100));
            prop_assert!(validator.validate(val).is_err());
        }

        #[test]
        fn range_above_max_fails(val in 101_i32..=i32::MAX) {
            let validator = RangeValidator::new(Some(10), Some(100));
            prop_assert!(validator.validate(val).is_err());
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn email_empty_is_empty_code() {
    let err = EmailValidator::validate("").unwrap_err();
    assert_eq!(err.code, ErrorCode::Empty);
}

#[test]
fn email_at_only_is_invalid() {
    assert!(EmailValidator::validate("@").is_err());
}

#[test]
fn email_no_tld_is_invalid() {
    assert!(EmailValidator::validate("user@domain").is_err());
}

#[test]
fn url_ftp_scheme_is_invalid() {
    assert!(UrlValidator::validate("ftp://example.com").is_err());
}

#[test]
fn url_no_scheme_is_invalid() {
    assert!(UrlValidator::validate("example.com").is_err());
}

#[test]
fn username_exactly_min_length() {
    assert!(UsernameValidator::validate("abc").is_ok());
}

#[test]
fn username_exactly_max_length() {
    let name = "a".repeat(32);
    assert!(UsernameValidator::validate(&name).is_ok());
}

#[test]
fn username_one_over_max() {
    let name = "a".repeat(33);
    assert!(UsernameValidator::validate(&name).is_err());
}

#[test]
fn password_exactly_min_length_valid() {
    assert!(PasswordValidator::validate("Abcdefg1").is_ok());
}

#[test]
fn password_no_uppercase_fails() {
    assert!(PasswordValidator::validate("lowercase1").is_err());
}

#[test]
fn password_no_lowercase_fails() {
    assert!(PasswordValidator::validate("UPPERCASE1").is_err());
}

#[test]
fn password_no_digit_fails() {
    assert!(PasswordValidator::validate("NoDigitHere").is_err());
}

#[test]
fn ipv4_all_zeros() {
    assert!(IpAddressValidator::validate_ipv4("0.0.0.0").is_ok());
}

#[test]
fn ipv4_all_max() {
    assert!(IpAddressValidator::validate_ipv4("255.255.255.255").is_ok());
}

#[test]
fn ipv4_non_numeric_octet() {
    assert!(IpAddressValidator::validate_ipv4("a.b.c.d").is_err());
}

#[test]
fn ipv4_empty_string() {
    assert!(IpAddressValidator::validate_ipv4("").is_err());
}

#[test]
fn ipv4_five_octets() {
    assert!(IpAddressValidator::validate_ipv4("1.2.3.4.5").is_err());
}

#[test]
fn range_no_bounds_always_passes() {
    let validator: RangeValidator<i32> = RangeValidator::new(None, None);
    assert!(validator.validate(0).is_ok());
    assert!(validator.validate(i32::MAX).is_ok());
    assert!(validator.validate(i32::MIN).is_ok());
}

#[test]
fn range_min_only() {
    let validator = RangeValidator::new(Some(5), None);
    assert!(validator.validate(5).is_ok());
    assert!(validator.validate(100).is_ok());
    assert!(validator.validate(4).is_err());
}

#[test]
fn range_max_only() {
    let validator = RangeValidator::new(None, Some(100));
    assert!(validator.validate(100).is_ok());
    assert!(validator.validate(0).is_ok());
    assert!(validator.validate(101).is_err());
}

#[test]
fn range_float_validation() {
    let validator = RangeValidator::new(Some(0.0_f64), Some(1.0));
    assert!(validator.validate(0.5).is_ok());
    assert!(validator.validate(0.0).is_ok());
    assert!(validator.validate(1.0).is_ok());
    assert!(validator.validate(-0.1).is_err());
    assert!(validator.validate(1.1).is_err());
}

#[test]
fn validator_error_display_format() {
    let err = ValidatorError {
        field: "age".to_string(),
        message: "must be positive".to_string(),
        code: ErrorCode::OutOfRange,
    };
    assert_eq!(format!("{err}"), "[OUT_OF_RANGE] age: must be positive");
}

#[test]
fn validator_error_is_std_error() {
    let err = ValidatorError {
        field: "x".to_string(),
        message: "y".to_string(),
        code: ErrorCode::Missing,
    };
    let _: &dyn std::error::Error = &err;
}
