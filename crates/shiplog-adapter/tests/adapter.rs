//! Integration tests for shiplog-adapter.

use shiplog_adapter::*;

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_adapter_output() {
    let adapter = Adapter::new(ConcreteAdaptee::new("hello world"));
    insta::assert_snapshot!("adapter_request_output", adapter.request());
}

#[test]
fn snapshot_named_adapter_output() {
    let adapter = NamedAdapter::new("github-ingest", ConcreteAdaptee::new("PR #42"));
    insta::assert_snapshot!("named_adapter_output", adapter.adapt());
}

#[test]
fn snapshot_named_adapter_debug() {
    let adapter = NamedAdapter::new("test-adapter", ConcreteAdaptee::new("debug-value"));
    insta::assert_snapshot!("named_adapter_debug", format!("{adapter:?}"));
}

#[test]
fn snapshot_builder_default_name() {
    let adapter = AdapterBuilder::default().build(ConcreteAdaptee::new("val"));
    insta::assert_snapshot!("builder_default_name_output", adapter.adapt());
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn adaptee_value_preserved(val in "[a-zA-Z0-9 ]{1,50}") {
            let adaptee = ConcreteAdaptee::new(&val);
            let output = adaptee.specific_request();
            prop_assert!(output.contains(&val));
        }

        #[test]
        fn adapter_wraps_adaptee(val in "[a-zA-Z0-9]{1,30}") {
            let adapter = Adapter::new(ConcreteAdaptee::new(&val));
            let output = adapter.request();
            prop_assert!(output.starts_with("Adapter:"));
            prop_assert!(output.contains(&val));
        }

        #[test]
        fn named_adapter_includes_name(
            name in "[a-z-]{1,20}",
            val in "[a-zA-Z0-9]{1,20}"
        ) {
            let adapter = NamedAdapter::new(&name, ConcreteAdaptee::new(&val));
            let output = adapter.adapt();
            prop_assert!(output.contains(&name));
            prop_assert!(output.contains(&val));
        }

        #[test]
        fn type_adapter_applies_converter(val in 0_i32..1000) {
            let adapter = TypeAdapter::new(val, |i: &i32| i.to_string());
            prop_assert_eq!(adapter.adapt(), val.to_string());
        }

        #[test]
        fn bidirectional_round_trip(val in 0_i64..10000) {
            let converter = BidirectionalConverter;
            let as_string = converter.adapt_from(&val);
            let back: i64 = converter.adapt_to(&as_string);
            prop_assert_eq!(back, val);
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn adaptee_empty_value() {
    let adaptee = ConcreteAdaptee::new("");
    assert_eq!(adaptee.specific_request(), "Adaptee: ");
}

#[test]
fn adapter_empty_adaptee() {
    let adapter = Adapter::new(ConcreteAdaptee::new(""));
    assert_eq!(adapter.request(), "Adapter: (Adaptee: ))");
}

#[test]
fn named_adapter_empty_name() {
    let adapter = NamedAdapter::new("", ConcreteAdaptee::new("val"));
    assert_eq!(adapter.name(), "");
    assert_eq!(adapter.adapt(), "[] Adaptee: val");
}

#[test]
fn builder_custom_name() {
    let adapter = AdapterBuilder::new()
        .name("custom")
        .build(ConcreteAdaptee::new("x"));
    assert_eq!(adapter.name(), "custom");
}

#[test]
fn builder_default_has_default_name() {
    let adapter = AdapterBuilder::new().build(ConcreteAdaptee::new("x"));
    assert_eq!(adapter.name(), "adapter");
}

#[test]
fn type_adapter_string_to_length() {
    let adapter = TypeAdapter::new("hello".to_string(), |s: &String| s.len());
    assert_eq!(adapter.adapt(), 5);
}

#[test]
fn type_adapter_identity() {
    let adapter = TypeAdapter::new(42, |i: &i32| *i);
    assert_eq!(adapter.adapt(), 42);
}

#[test]
fn generic_converter_string_to_uppercase() {
    let converter = GenericConverter::new(|s: &String| s.to_uppercase());
    assert_eq!(converter.convert(&"hello".to_string()), "HELLO");
}

#[test]
fn generic_converter_with_empty() {
    let converter = GenericConverter::new(|s: &String| s.len());
    assert_eq!(converter.convert(&String::new()), 0);
}

#[test]
fn bidirectional_non_numeric_string_defaults_to_zero() {
    let converter = BidirectionalConverter;
    assert_eq!(converter.adapt_to(&"not a number".to_string()), 0);
}

#[test]
fn bidirectional_negative_number() {
    let converter = BidirectionalConverter;
    let s = converter.adapt_from(&-42);
    assert_eq!(s, "-42");
    assert_eq!(converter.adapt_to(&s), -42);
}

#[test]
fn bidirectional_zero() {
    let converter = BidirectionalConverter;
    assert_eq!(converter.adapt_to(&"0".to_string()), 0);
    assert_eq!(converter.adapt_from(&0), "0");
}

#[test]
fn adapter_builder_is_debug() {
    let builder = AdapterBuilder::new();
    let debug = format!("{builder:?}");
    assert!(debug.contains("AdapterBuilder"));
}
