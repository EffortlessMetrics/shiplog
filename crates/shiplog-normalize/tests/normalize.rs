use proptest::prelude::*;
use shiplog_normalize::{
    normalize_bool, normalize_line_endings, normalize_number, normalize_path, normalize_string,
    normalize_whitespace,
};

// ── normalize_string ───────────────────────────────────────────────

#[test]
fn string_basic() {
    assert_eq!(normalize_string("  Hello World  "), "hello world");
}

#[test]
fn string_empty() {
    assert_eq!(normalize_string(""), "");
}

#[test]
fn string_already_normalized() {
    assert_eq!(normalize_string("hello"), "hello");
}

#[test]
fn string_unicode() {
    assert_eq!(normalize_string("  ÜBER  "), "über");
}

#[test]
fn string_only_whitespace() {
    assert_eq!(normalize_string("   "), "");
}

// ── normalize_path ─────────────────────────────────────────────────

#[test]
fn path_collapses_slashes() {
    assert_eq!(normalize_path("foo///bar//baz"), "foo/bar/baz");
}

#[test]
fn path_removes_trailing_slash() {
    assert_eq!(normalize_path("/foo/bar/"), "/foo/bar");
}

#[test]
fn path_root_preserved() {
    assert_eq!(normalize_path("/"), "/");
}

#[test]
fn path_empty() {
    assert_eq!(normalize_path(""), "");
}

#[test]
fn path_single_segment() {
    assert_eq!(normalize_path("foo"), "foo");
}

#[test]
fn path_many_slashes() {
    assert_eq!(normalize_path("////////"), "/");
}

// ── normalize_whitespace ───────────────────────────────────────────

#[test]
fn whitespace_collapses() {
    assert_eq!(normalize_whitespace("a   b   c"), "a b c");
}

#[test]
fn whitespace_trims() {
    assert_eq!(normalize_whitespace("  hello  "), "hello");
}

#[test]
fn whitespace_empty() {
    assert_eq!(normalize_whitespace(""), "");
}

#[test]
fn whitespace_tabs_and_newlines() {
    assert_eq!(normalize_whitespace("a\t\tb\n\nc"), "a b c");
}

#[test]
fn whitespace_only_spaces() {
    assert_eq!(normalize_whitespace("     "), "");
}

#[test]
fn whitespace_single_word() {
    assert_eq!(normalize_whitespace("hello"), "hello");
}

// ── normalize_line_endings ─────────────────────────────────────────

#[test]
fn line_endings_crlf() {
    assert_eq!(normalize_line_endings("a\r\nb\r\nc"), "a\nb\nc");
}

#[test]
fn line_endings_cr() {
    assert_eq!(normalize_line_endings("a\rb\rc"), "a\nb\nc");
}

#[test]
fn line_endings_lf_unchanged() {
    assert_eq!(normalize_line_endings("a\nb\nc"), "a\nb\nc");
}

#[test]
fn line_endings_mixed() {
    assert_eq!(normalize_line_endings("a\r\nb\rc\nd"), "a\nb\nc\nd");
}

#[test]
fn line_endings_empty() {
    assert_eq!(normalize_line_endings(""), "");
}

// ── normalize_bool ─────────────────────────────────────────────────

#[test]
fn bool_truthy_values() {
    for v in ["true", "1", "yes", "on", "enabled", "TRUE", "Yes", "ON"] {
        assert_eq!(normalize_bool(v), "true", "failed for {v}");
    }
}

#[test]
fn bool_falsy_values() {
    for v in ["false", "0", "no", "off", "disabled", "anything", ""] {
        assert_eq!(normalize_bool(v), "false", "failed for {v}");
    }
}

#[test]
fn bool_whitespace_trimmed() {
    assert_eq!(normalize_bool("  true  "), "true");
    assert_eq!(normalize_bool("  false  "), "false");
}

// ── normalize_number ───────────────────────────────────────────────

#[test]
fn number_leading_zeros() {
    assert_eq!(normalize_number("00123"), "123");
}

#[test]
fn number_decimal_trailing_zeros() {
    assert_eq!(normalize_number("1.2300"), "1.23");
}

#[test]
fn number_negative_leading_zeros() {
    assert_eq!(normalize_number("-007"), "-7");
}

#[test]
fn number_zero() {
    assert_eq!(normalize_number("0"), "0");
    assert_eq!(normalize_number("000"), "0");
    assert_eq!(normalize_number("0.0"), "0");
    assert_eq!(normalize_number("0.00"), "0");
}

#[test]
fn number_just_decimal() {
    assert_eq!(normalize_number("1.0"), "1");
}

#[test]
fn number_negative_zero() {
    // -0 normalizes to "-0"
    let result = normalize_number("-0");
    assert!(result == "-0" || result == "0");
}

#[test]
fn number_whitespace_trimmed() {
    assert_eq!(normalize_number("  42  "), "42");
}

// ── Property tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_normalize_string_idempotent(s in "\\PC{0,50}") {
        let once = normalize_string(&s);
        let twice = normalize_string(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_normalize_string_always_lowercase(s in "\\PC{0,50}") {
        let result = normalize_string(&s);
        prop_assert_eq!(result.clone(), result.to_lowercase());
    }

    #[test]
    fn prop_normalize_string_no_leading_trailing_ws(s in "\\PC{0,50}") {
        let result = normalize_string(&s);
        prop_assert_eq!(result.clone(), result.trim().to_string());
    }

    #[test]
    fn prop_normalize_path_idempotent(s in "[a-z/]{0,30}") {
        let once = normalize_path(&s);
        let twice = normalize_path(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_normalize_path_no_double_slashes(s in "[a-z/]{0,30}") {
        let result = normalize_path(&s);
        prop_assert!(!result.contains("//"));
    }

    #[test]
    fn prop_normalize_whitespace_idempotent(s in "\\PC{0,50}") {
        let once = normalize_whitespace(&s);
        let twice = normalize_whitespace(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_normalize_line_endings_idempotent(s in "\\PC{0,50}") {
        let once = normalize_line_endings(&s);
        let twice = normalize_line_endings(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_normalize_line_endings_no_cr(s in "\\PC{0,50}") {
        let result = normalize_line_endings(&s);
        prop_assert!(!result.contains('\r'));
    }

    #[test]
    fn prop_normalize_bool_only_true_or_false(s in "\\PC{0,20}") {
        let result = normalize_bool(&s);
        prop_assert!(result == "true" || result == "false");
    }

    #[test]
    fn prop_normalize_number_idempotent(n in -10000i64..10000) {
        let s = n.to_string();
        let once = normalize_number(&s);
        let twice = normalize_number(&once);
        prop_assert_eq!(once, twice);
    }
}
