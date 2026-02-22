//! Serialization/deserialization utilities for shiplog.
//!
//! This crate provides utilities for serializing and deserializing shiplog data
//! in various formats (JSON, YAML).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Serialized data format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum DataFormat {
    #[default]
    Json,
    Yaml,
}

impl fmt::Display for DataFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataFormat::Json => write!(f, "json"),
            DataFormat::Yaml => write!(f, "yaml"),
        }
    }
}

/// Serde configuration for serialization
#[derive(Debug, Clone)]
pub struct SerdeConfig {
    pub format: DataFormat,
    pub pretty: bool,
    pub flatten: bool,
}

impl Default for SerdeConfig {
    fn default() -> Self {
        Self {
            format: DataFormat::Json,
            pretty: true,
            flatten: false,
        }
    }
}

/// Serialize data to JSON string
pub fn to_json<T: Serialize>(value: &T) -> anyhow::Result<String> {
    let config = SerdeConfig::default();
    to_json_with_config(value, &config)
}

/// Serialize data to JSON string with custom config
pub fn to_json_with_config<T: Serialize>(
    value: &T,
    config: &SerdeConfig,
) -> anyhow::Result<String> {
    match config.format {
        DataFormat::Json => if config.pretty {
            serde_json::to_string_pretty(value)
        } else {
            serde_json::to_string(value)
        }
        .map_err(|e| anyhow::anyhow!("JSON serialization failed: {}", e)),
        DataFormat::Yaml => serde_yaml::to_string(value)
            .map_err(|e| anyhow::anyhow!("YAML serialization failed: {}", e)),
    }
}

/// Deserialize data from JSON string
pub fn from_json<T: for<'de> Deserialize<'de>>(json: &str) -> anyhow::Result<T> {
    serde_json::from_str(json).map_err(|e| anyhow::anyhow!("JSON deserialization failed: {}", e))
}

/// Serialize data to YAML string
pub fn to_yaml<T: Serialize>(value: &T) -> anyhow::Result<String> {
    serde_yaml::to_string(value).map_err(|e| anyhow::anyhow!("YAML serialization failed: {}", e))
}

/// Deserialize data from YAML string
pub fn from_yaml<T: for<'de> Deserialize<'de>>(yaml: &str) -> anyhow::Result<T> {
    serde_yaml::from_str(yaml).map_err(|e| anyhow::anyhow!("YAML deserialization failed: {}", e))
}

/// Serialize/deserialize wrapper for flexible format handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlexibleData {
    pub format: DataFormat,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

impl FlexibleData {
    pub fn new(format: DataFormat, content: String) -> Self {
        Self {
            format,
            content,
            metadata: HashMap::new(),
        }
    }

    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    pub fn deserialize<T: for<'de> Deserialize<'de>>(&self) -> anyhow::Result<T> {
        match self.format {
            DataFormat::Json => from_json(&self.content),
            DataFormat::Yaml => from_yaml(&self.content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_serde_config_default() {
        let config = SerdeConfig::default();
        assert_eq!(config.format, DataFormat::Json);
        assert!(config.pretty);
        assert!(!config.flatten);
    }

    #[test]
    fn test_data_format_display() {
        assert_eq!(format!("{}", DataFormat::Json), "json");
        assert_eq!(format!("{}", DataFormat::Yaml), "yaml");
    }

    #[test]
    fn test_to_json() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        let json = to_json(&data).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_from_json() {
        let json = r#"{"name":"test","value":42}"#;
        let data: TestData = from_json(json).unwrap();
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_to_yaml() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        let yaml = to_yaml(&data).unwrap();
        assert!(yaml.contains("name: test"));
        assert!(yaml.contains("value: 42"));
    }

    #[test]
    fn test_from_yaml() {
        let yaml = "name: test\nvalue: 42\n";
        let data: TestData = from_yaml(yaml).unwrap();
        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_flexible_data_json() {
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };
        let json = to_json(&data).unwrap();
        let flex = FlexibleData::new(DataFormat::Json, json);

        let deserialized: TestData = flex.deserialize().unwrap();
        assert_eq!(deserialized.name, "test");
        assert_eq!(deserialized.value, 42);
    }

    #[test]
    fn test_flexible_data_with_metadata() {
        let flex = FlexibleData::new(DataFormat::Json, "{}".to_string())
            .with_metadata("source".to_string(), "test".to_string());

        assert_eq!(flex.metadata.get("source"), Some(&"test".to_string()));
    }

    #[test]
    fn test_roundtrip_json() {
        let original = TestData {
            name: "roundtrip".to_string(),
            value: 123,
        };

        let json = to_json(&original).unwrap();
        let deserialized: TestData = from_json(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_roundtrip_yaml() {
        let original = TestData {
            name: "roundtrip".to_string(),
            value: 456,
        };

        let yaml = to_yaml(&original).unwrap();
        let deserialized: TestData = from_yaml(&yaml).unwrap();

        assert_eq!(original, deserialized);
    }
}
