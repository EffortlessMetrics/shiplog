//! Integration tests for shiplog-parse.

use proptest::prelude::*;
use shiplog_parse::*;

// ── Known-answer tests ──────────────────────────────────────────

#[test]
fn parse_u64_valid() {
    assert_eq!(parse_u64("0").unwrap(), 0);
    assert_eq!(parse_u64("123").unwrap(), 123);
    assert_eq!(parse_u64("18446744073709551615").unwrap(), u64::MAX);
}

#[test]
fn parse_u64_whitespace() {
    assert_eq!(parse_u64("  42  ").unwrap(), 42);
    assert_eq!(parse_u64("\t99\n").unwrap(), 99);
}

#[test]
fn parse_u64_errors() {
    assert!(parse_u64("").is_err());
    assert!(parse_u64("abc").is_err());
    assert!(parse_u64("-1").is_err());
    assert!(parse_u64("18446744073709551616").is_err()); // overflow
    assert!(parse_u64("1.5").is_err());
}

#[test]
fn parse_i64_valid() {
    assert_eq!(parse_i64("0").unwrap(), 0);
    assert_eq!(parse_i64("-123").unwrap(), -123);
    assert_eq!(parse_i64("9223372036854775807").unwrap(), i64::MAX);
    assert_eq!(parse_i64("-9223372036854775808").unwrap(), i64::MIN);
}

#[test]
fn parse_i64_errors() {
    assert!(parse_i64("").is_err());
    assert!(parse_i64("not_a_number").is_err());
    assert!(parse_i64("9223372036854775808").is_err()); // overflow
}

#[test]
fn parse_f64_valid() {
    assert!((parse_f64("1.234").unwrap() - 1.234).abs() < 0.001);
    assert!((parse_f64("-0.5").unwrap() - (-0.5)).abs() < f64::EPSILON);
    assert!((parse_f64("0").unwrap()).abs() < f64::EPSILON);
    assert!(parse_f64("inf").unwrap().is_infinite());
    assert!(parse_f64("NaN").unwrap().is_nan());
}

#[test]
fn parse_f64_errors() {
    assert!(parse_f64("").is_err());
    assert!(parse_f64("abc").is_err());
}

#[test]
fn parse_bool_truthy() {
    for input in &["true", "TRUE", "True", "1", "yes", "YES", "on", "ON"] {
        assert!(parse_bool(input).unwrap(), "Expected true for {}", input);
    }
}

#[test]
fn parse_bool_falsy() {
    for input in &["false", "FALSE", "False", "0", "no", "NO", "off", "OFF"] {
        assert!(!parse_bool(input).unwrap(), "Expected false for {}", input);
    }
}

#[test]
fn parse_bool_errors() {
    assert!(parse_bool("").is_err());
    assert!(parse_bool("maybe").is_err());
    assert!(parse_bool("2").is_err());
    assert!(parse_bool("yep").is_err());
}

#[test]
fn parse_bool_whitespace() {
    assert!(parse_bool("  true  ").unwrap());
    assert!(!parse_bool("  false  ").unwrap());
}

// ── Comma-separated tests ───────────────────────────────────────

#[test]
fn parse_comma_separated_basic() {
    assert_eq!(parse_comma_separated("a,b,c"), vec!["a", "b", "c"]);
}

#[test]
fn parse_comma_separated_whitespace() {
    assert_eq!(parse_comma_separated(" a , b , c "), vec!["a", "b", "c"]);
}

#[test]
fn parse_comma_separated_empty_string() {
    let result = parse_comma_separated("");
    assert!(result.is_empty());
}

#[test]
fn parse_comma_separated_single_item() {
    assert_eq!(parse_comma_separated("hello"), vec!["hello"]);
}

#[test]
fn parse_comma_separated_trailing_comma() {
    assert_eq!(parse_comma_separated("a,b,"), vec!["a", "b"]);
}

#[test]
fn parse_comma_separated_leading_comma() {
    assert_eq!(parse_comma_separated(",a,b"), vec!["a", "b"]);
}

#[test]
fn parse_comma_separated_only_commas() {
    let result = parse_comma_separated(",,,");
    assert!(result.is_empty());
}

// ── Key-value tests ─────────────────────────────────────────────

#[test]
fn parse_key_value_basic() {
    assert_eq!(
        parse_key_value("key=value").unwrap(),
        ("key".to_string(), "value".to_string())
    );
}

#[test]
fn parse_key_value_equals_in_value() {
    assert_eq!(
        parse_key_value("key=a=b=c").unwrap(),
        ("key".to_string(), "a=b=c".to_string())
    );
}

#[test]
fn parse_key_value_whitespace() {
    assert_eq!(
        parse_key_value("  key  =  value  ").unwrap(),
        ("key".to_string(), "value".to_string())
    );
}

#[test]
fn parse_key_value_empty_value() {
    assert_eq!(
        parse_key_value("key=").unwrap(),
        ("key".to_string(), "".to_string())
    );
}

#[test]
fn parse_key_value_no_equals() {
    assert!(parse_key_value("noequals").is_err());
}

// ── Trim / unquote tests ────────────────────────────────────────

#[test]
fn trim_various() {
    assert_eq!(trim(""), "");
    assert_eq!(trim("   "), "");
    assert_eq!(trim("  hello  "), "hello");
    assert_eq!(trim("\t\nhello\t\n"), "hello");
}

#[test]
fn unquote_double_quotes() {
    assert_eq!(unquote("\"hello\""), "hello");
}

#[test]
fn unquote_single_quotes() {
    assert_eq!(unquote("'hello'"), "hello");
}

#[test]
fn unquote_no_quotes() {
    assert_eq!(unquote("hello"), "hello");
}

#[test]
fn unquote_mismatched_quotes() {
    assert_eq!(unquote("\"hello'"), "\"hello'");
}

#[test]
fn unquote_empty_quotes() {
    assert_eq!(unquote("\"\""), "");
    assert_eq!(unquote("''"), "");
}

#[test]
fn unquote_whitespace_with_quotes() {
    assert_eq!(unquote("  \"hello\"  "), "hello");
}

// ── Unicode edge cases ──────────────────────────────────────────

#[test]
fn trim_unicode() {
    assert_eq!(trim("  日本語  "), "日本語");
}

#[test]
fn parse_comma_separated_unicode() {
    assert_eq!(
        parse_comma_separated("日本,中国,韓国"),
        vec!["日本", "中国", "韓国"]
    );
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_parse_u64_roundtrip(n: u64) {
        let s = n.to_string();
        prop_assert_eq!(parse_u64(&s).unwrap(), n);
    }

    #[test]
    fn prop_parse_i64_roundtrip(n: i64) {
        let s = n.to_string();
        prop_assert_eq!(parse_i64(&s).unwrap(), n);
    }

    #[test]
    fn prop_parse_f64_roundtrip_finite(n in proptest::num::f64::NORMAL) {
        let s = format!("{}", n);
        let parsed = parse_f64(&s);
        prop_assert!(parsed.is_ok());
    }

    #[test]
    fn prop_trim_is_idempotent(s in "\\PC{0,50}") {
        let once = trim(&s);
        let twice = trim(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_trim_no_leading_trailing_whitespace(s in "\\PC{0,50}") {
        let trimmed = trim(&s);
        if !trimmed.is_empty() {
            prop_assert!(!trimmed.starts_with(char::is_whitespace));
            prop_assert!(!trimmed.ends_with(char::is_whitespace));
        }
    }

    #[test]
    fn prop_unquote_idempotent_after_first(s in "[a-zA-Z0-9]{0,20}") {
        let quoted = format!("\"{}\"", s);
        let once = unquote(&quoted);
        let twice = unquote(&once);
        // After first unquote we get the inner string; second unquote should be no-op
        // (unless the inner string itself looks quoted)
        prop_assert_eq!(once, s);
        let _ = twice; // just verify it doesn't panic
    }

    #[test]
    fn prop_parse_comma_separated_count(
        items in proptest::collection::vec("[a-zA-Z0-9]+", 1..10)
    ) {
        let joined = items.join(",");
        let parsed = parse_comma_separated(&joined);
        prop_assert_eq!(parsed.len(), items.len());
    }

    #[test]
    fn prop_parse_key_value_roundtrip(
        key in "[a-zA-Z][a-zA-Z0-9]{0,10}",
        value in "[a-zA-Z0-9]{0,20}"
    ) {
        let input = format!("{}={}", key, value);
        let (k, v) = parse_key_value(&input).unwrap();
        prop_assert_eq!(k, key);
        prop_assert_eq!(v, value);
    }
}
