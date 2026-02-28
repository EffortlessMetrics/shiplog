//! Integration tests for shiplog-error.

use shiplog_error::*;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_parse_error_display() {
    let err = parse_error("unexpected token at line 5");
    insta::assert_snapshot!("parse_error_display", format!("{err}"));
}

#[test]
fn snapshot_validation_error_with_context() {
    let err = validation_error("field is required")
        .with_context("field", "email")
        .with_context("form", "registration");
    insta::assert_snapshot!("validation_error_with_context", format!("{err}"));
}

#[test]
fn snapshot_error_with_source() {
    let source = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let err = ShiplogError::with_source("cannot read config", ErrorCategory::Io, Box::new(source));
    insta::assert_snapshot!("error_with_source", format!("{err}"));
}

#[test]
fn snapshot_builder_complex_error() {
    let err = ErrorBuilder::new("request failed", ErrorCategory::Network)
        .with_context("url", "https://api.github.com/repos")
        .with_context("status", "429")
        .with_context("retry_after", "60s")
        .build();
    insta::assert_snapshot!("builder_complex_error", format!("{err}"));
}

#[test]
fn snapshot_all_category_displays() {
    let categories = [
        ErrorCategory::Parse,
        ErrorCategory::Validation,
        ErrorCategory::Io,
        ErrorCategory::Config,
        ErrorCategory::Network,
        ErrorCategory::Authentication,
        ErrorCategory::RateLimit,
        ErrorCategory::Timeout,
        ErrorCategory::Unknown,
    ];
    let displays: Vec<String> = categories.iter().map(|c| format!("{c}")).collect();
    insta::assert_yaml_snapshot!("all_category_displays", displays);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn error_message_preserved(msg in "[a-zA-Z0-9 ]{1,100}") {
            let err = ShiplogError::new(&msg, ErrorCategory::Parse);
            prop_assert_eq!(err.message(), &msg);
        }

        #[test]
        fn error_display_contains_category_and_message(msg in "[a-zA-Z0-9 ]{1,50}") {
            let err = ShiplogError::new(&msg, ErrorCategory::Validation);
            let display = format!("{err}");
            prop_assert!(display.contains("validation"));
            prop_assert!(display.contains(&msg));
        }

        #[test]
        fn context_entries_preserved(
            key in "[a-z]{1,10}",
            value in "[a-z]{1,10}"
        ) {
            let err = ShiplogError::new("test", ErrorCategory::Config)
                .with_context(&key, &value);
            let ctx = err.context();
            prop_assert_eq!(ctx.len(), 1);
            prop_assert_eq!(&ctx[0].0, &key);
            prop_assert_eq!(&ctx[0].1, &value);
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_message_error() {
    let err = ShiplogError::new("", ErrorCategory::Unknown);
    assert_eq!(err.message(), "");
    let display = format!("{err}");
    assert!(display.contains("[unknown]"));
}

#[test]
fn many_context_entries() {
    let mut err = ShiplogError::new("multi-context", ErrorCategory::Config);
    for i in 0..10 {
        err = err.with_context(format!("key{i}"), format!("val{i}"));
    }
    assert_eq!(err.context().len(), 10);
    let display = format!("{err}");
    assert!(display.contains("key0=val0"));
    assert!(display.contains("key9=val9"));
}

#[test]
fn error_category_equality() {
    assert_eq!(ErrorCategory::Parse, ErrorCategory::Parse);
    assert_ne!(ErrorCategory::Parse, ErrorCategory::Io);
}

#[test]
fn error_is_category_checks() {
    assert!(parse_error("x").is_parse_error());
    assert!(!parse_error("x").is_validation_error());
    assert!(!parse_error("x").is_io_error());

    assert!(validation_error("x").is_validation_error());
    assert!(!validation_error("x").is_parse_error());

    assert!(io_error("x").is_io_error());
    assert!(!io_error("x").is_parse_error());
}

#[test]
fn convenience_functions_produce_correct_categories() {
    assert_eq!(parse_error("x").category(), &ErrorCategory::Parse);
    assert_eq!(validation_error("x").category(), &ErrorCategory::Validation);
    assert_eq!(io_error("x").category(), &ErrorCategory::Io);
    assert_eq!(config_error("x").category(), &ErrorCategory::Config);
    assert_eq!(network_error("x").category(), &ErrorCategory::Network);
}

#[test]
fn builder_without_source() {
    let err = ErrorBuilder::new("no source", ErrorCategory::Parse).build();
    assert!(std::error::Error::source(&err).is_none());
}

#[test]
fn builder_with_source() {
    let source = std::io::Error::other("underlying");
    let err = ErrorBuilder::new("wrapped", ErrorCategory::Io)
        .with_source(Box::new(source))
        .build();
    assert!(std::error::Error::source(&err).is_some());
}

#[test]
fn anyhow_conversion_preserves_message() {
    let anyhow_err = anyhow::anyhow!("original message");
    let err: ShiplogError = anyhow_err.into();
    assert!(err.message().contains("original message"));
    assert_eq!(err.category(), &ErrorCategory::Unknown);
}

#[test]
fn error_display_no_context_no_source() {
    let err = ShiplogError::new("simple", ErrorCategory::Parse);
    assert_eq!(format!("{err}"), "[parse] simple");
}

#[test]
fn error_display_with_context_no_source() {
    let err = ShiplogError::new("msg", ErrorCategory::Io).with_context("file", "test.txt");
    assert_eq!(format!("{err}"), "[io] msg (file=test.txt)");
}

#[test]
fn error_display_multiple_context_pairs() {
    let err = ShiplogError::new("msg", ErrorCategory::Network)
        .with_context("host", "example.com")
        .with_context("port", "443");
    let display = format!("{err}");
    assert!(display.contains("host=example.com, port=443"));
}

#[test]
fn error_category_clone() {
    let cat = ErrorCategory::RateLimit;
    let cloned = cat.clone();
    assert_eq!(cat, cloned);
}

#[test]
fn error_category_hash() {
    use std::collections::HashSet;
    let mut set = HashSet::new();
    set.insert(ErrorCategory::Parse);
    set.insert(ErrorCategory::Parse);
    set.insert(ErrorCategory::Io);
    assert_eq!(set.len(), 2);
}
