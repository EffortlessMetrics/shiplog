//! Comprehensive tests for shiplog-serde: round-trip, edge cases, property tests.

use proptest::prelude::*;
use shiplog_serde::{
    DataFormat, FlexibleData, SerdeConfig, from_json, from_yaml, to_json, to_json_with_config,
    to_yaml,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── helpers ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Simple {
    name: String,
    value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Nested {
    label: String,
    inner: Vec<Simple>,
    tags: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct WithOptionals {
    required: String,
    optional_str: Option<String>,
    optional_num: Option<f64>,
}

// ── JSON round-trip ─────────────────────────────────────────────────────────

#[test]
fn json_roundtrip_simple() {
    let orig = Simple {
        name: "test".into(),
        value: 42,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_nested() {
    let mut tags = HashMap::new();
    tags.insert("env".into(), "prod".into());
    let orig = Nested {
        label: "parent".into(),
        inner: vec![
            Simple {
                name: "a".into(),
                value: 1,
            },
            Simple {
                name: "b".into(),
                value: -999,
            },
        ],
        tags,
    };
    let json = to_json(&orig).unwrap();
    let back: Nested = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_optionals_present() {
    let orig = WithOptionals {
        required: "x".into(),
        optional_str: Some("y".into()),
        optional_num: Some(1.23),
    };
    let json = to_json(&orig).unwrap();
    let back: WithOptionals = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_optionals_absent() {
    let orig = WithOptionals {
        required: "x".into(),
        optional_str: None,
        optional_num: None,
    };
    let json = to_json(&orig).unwrap();
    let back: WithOptionals = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

// ── YAML round-trip ─────────────────────────────────────────────────────────

#[test]
fn yaml_roundtrip_simple() {
    let orig = Simple {
        name: "test".into(),
        value: -7,
    };
    let yaml = to_yaml(&orig).unwrap();
    let back: Simple = from_yaml(&yaml).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn yaml_roundtrip_nested() {
    let orig = Nested {
        label: "root".into(),
        inner: vec![],
        tags: HashMap::new(),
    };
    let yaml = to_yaml(&orig).unwrap();
    let back: Nested = from_yaml(&yaml).unwrap();
    assert_eq!(orig, back);
}

// ── Cross-format ────────────────────────────────────────────────────────────

#[test]
fn json_to_yaml_to_json() {
    let orig = Simple {
        name: "cross".into(),
        value: 100,
    };
    let json = to_json(&orig).unwrap();
    let mid: Simple = from_json(&json).unwrap();
    let yaml = to_yaml(&mid).unwrap();
    let back: Simple = from_yaml(&yaml).unwrap();
    assert_eq!(orig, back);
}

// ── SerdeConfig variations ──────────────────────────────────────────────────

#[test]
fn config_compact_json() {
    let cfg = SerdeConfig {
        format: DataFormat::Json,
        pretty: false,
        flatten: false,
    };
    let data = Simple {
        name: "c".into(),
        value: 0,
    };
    let out = to_json_with_config(&data, &cfg).unwrap();
    // Compact JSON has no newlines
    assert!(!out.contains('\n'));
    let back: Simple = from_json(&out).unwrap();
    assert_eq!(data, back);
}

#[test]
fn config_pretty_json() {
    let cfg = SerdeConfig {
        format: DataFormat::Json,
        pretty: true,
        flatten: false,
    };
    let data = Simple {
        name: "p".into(),
        value: 1,
    };
    let out = to_json_with_config(&data, &cfg).unwrap();
    assert!(out.contains('\n'));
}

#[test]
fn config_yaml_ignores_pretty_flag() {
    let cfg = SerdeConfig {
        format: DataFormat::Yaml,
        pretty: true,
        flatten: false,
    };
    let data = Simple {
        name: "y".into(),
        value: 2,
    };
    let out = to_json_with_config(&data, &cfg).unwrap();
    let back: Simple = from_yaml(&out).unwrap();
    assert_eq!(data, back);
}

#[test]
fn serde_config_default_is_pretty_json() {
    let cfg = SerdeConfig::default();
    assert_eq!(cfg.format, DataFormat::Json);
    assert!(cfg.pretty);
    assert!(!cfg.flatten);
}

// ── DataFormat ──────────────────────────────────────────────────────────────

#[test]
fn data_format_display() {
    assert_eq!(DataFormat::Json.to_string(), "json");
    assert_eq!(DataFormat::Yaml.to_string(), "yaml");
}

#[test]
fn data_format_default_is_json() {
    assert_eq!(DataFormat::default(), DataFormat::Json);
}

#[test]
fn data_format_serde_roundtrip() {
    let orig = DataFormat::Yaml;
    let json = serde_json::to_string(&orig).unwrap();
    let back: DataFormat = serde_json::from_str(&json).unwrap();
    assert_eq!(orig, back);
}

// ── FlexibleData ────────────────────────────────────────────────────────────

#[test]
fn flexible_data_json_deserialize() {
    let data = Simple {
        name: "flex".into(),
        value: 10,
    };
    let json = to_json(&data).unwrap();
    let flex = FlexibleData::new(DataFormat::Json, json);
    let back: Simple = flex.deserialize().unwrap();
    assert_eq!(data, back);
}

#[test]
fn flexible_data_yaml_deserialize() {
    let data = Simple {
        name: "flex".into(),
        value: 20,
    };
    let yaml = to_yaml(&data).unwrap();
    let flex = FlexibleData::new(DataFormat::Yaml, yaml);
    let back: Simple = flex.deserialize().unwrap();
    assert_eq!(data, back);
}

#[test]
fn flexible_data_metadata_accumulates() {
    let flex = FlexibleData::new(DataFormat::Json, "{}".into())
        .with_metadata("k1".into(), "v1".into())
        .with_metadata("k2".into(), "v2".into());
    assert_eq!(flex.metadata.len(), 2);
    assert_eq!(flex.metadata["k1"], "v1");
    assert_eq!(flex.metadata["k2"], "v2");
}

#[test]
fn flexible_data_metadata_overwrites_same_key() {
    let flex = FlexibleData::new(DataFormat::Json, "{}".into())
        .with_metadata("k".into(), "first".into())
        .with_metadata("k".into(), "second".into());
    assert_eq!(flex.metadata.len(), 1);
    assert_eq!(flex.metadata["k"], "second");
}

#[test]
fn flexible_data_serde_roundtrip() {
    let flex = FlexibleData::new(DataFormat::Json, r#"{"name":"x","value":1}"#.into())
        .with_metadata("src".into(), "test".into());
    let json = serde_json::to_string(&flex).unwrap();
    let back: FlexibleData = serde_json::from_str(&json).unwrap();
    assert_eq!(back.content, flex.content);
    assert_eq!(back.metadata, flex.metadata);
}

// ── Error handling ──────────────────────────────────────────────────────────

#[test]
fn from_json_invalid_returns_error() {
    let result: Result<Simple, _> = from_json("{broken");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("JSON deserialization"));
}

#[test]
fn from_yaml_invalid_returns_error() {
    let result: Result<Simple, _> = from_yaml(":\n  :\n  invalid: [[[");
    assert!(result.is_err());
}

#[test]
fn from_json_wrong_type_returns_error() {
    let result: Result<Simple, _> = from_json(r#"{"name":"ok","value":"not_a_number"}"#);
    assert!(result.is_err());
}

#[test]
fn from_json_empty_string_returns_error() {
    let result: Result<Simple, _> = from_json("");
    assert!(result.is_err());
}

#[test]
fn flexible_data_wrong_format_returns_error() {
    // JSON content declared as YAML
    let flex = FlexibleData::new(DataFormat::Json, "name: test\nvalue: 1\n".into());
    let result: Result<Simple, _> = flex.deserialize();
    // JSON parser will fail on YAML-formatted content
    assert!(result.is_err());
}

// ── Unicode / special chars ─────────────────────────────────────────────────

#[test]
fn json_roundtrip_unicode() {
    let orig = Simple {
        name: "日本語テスト 🚀 émojis".into(),
        value: 0,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn yaml_roundtrip_unicode() {
    let orig = Simple {
        name: "中文测试 αβγ".into(),
        value: i64::MIN,
    };
    let yaml = to_yaml(&orig).unwrap();
    let back: Simple = from_yaml(&yaml).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_special_chars() {
    let orig = Simple {
        name: r#"quotes "inside" and \ backslash"#.into(),
        value: 0,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_empty_string() {
    let orig = Simple {
        name: "".into(),
        value: 0,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

// ── Boundary values ─────────────────────────────────────────────────────────

#[test]
fn json_roundtrip_extreme_numbers() {
    let orig = Simple {
        name: "max".into(),
        value: i64::MAX,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);

    let orig = Simple {
        name: "min".into(),
        value: i64::MIN,
    };
    let json = to_json(&orig).unwrap();
    let back: Simple = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

#[test]
fn json_roundtrip_empty_collections() {
    let orig = Nested {
        label: "empty".into(),
        inner: vec![],
        tags: HashMap::new(),
    };
    let json = to_json(&orig).unwrap();
    let back: Nested = from_json(&json).unwrap();
    assert_eq!(orig, back);
}

// ── Property tests ──────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_json_roundtrip(name in ".*", value in proptest::num::i64::ANY) {
        let orig = Simple { name: name.clone(), value };
        let json = to_json(&orig).unwrap();
        let back: Simple = from_json(&json).unwrap();
        prop_assert_eq!(orig, back);
    }

    #[test]
    fn prop_yaml_roundtrip(name in "[a-zA-Z0-9_ ]{0,100}", value in proptest::num::i64::ANY) {
        let orig = Simple { name: name.clone(), value };
        let yaml = to_yaml(&orig).unwrap();
        let back: Simple = from_yaml(&yaml).unwrap();
        prop_assert_eq!(orig, back);
    }

    #[test]
    fn prop_compact_json_has_no_leading_newlines(name in "[a-z]{1,20}", value in 0i64..1000) {
        let cfg = SerdeConfig { format: DataFormat::Json, pretty: false, flatten: false };
        let data = Simple { name, value };
        let out = to_json_with_config(&data, &cfg).unwrap();
        prop_assert!(!out.starts_with('\n'));
    }
}
