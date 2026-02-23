//! Fuzz harness for plugin manifests
//!
//! This harness tests robustness of plugin manifest parser against
//! malformed or unexpected plugin manifest data.
//! Target: Plugin manifest YAML/JSON format

#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;

// Plugin manifest structures
#[derive(Debug, Deserialize)]
struct PluginManifest {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    min_shiplog: Option<String>,
    #[serde(default)]
    api_version: Option<String>,
    #[serde(default)]
    schema_version: Option<String>,
    #[serde(default)]
    capabilities: Option<Vec<String>>,
    #[serde(default)]
    dependencies: Option<Vec<String>>,
    #[serde(default)]
    author: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PluginConfig {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    args: Option<Vec<String>>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    settings: Option<serde_json::Value>,
}

fuzz_target!(|data: &[u8]| {
    // Convert bytes to string, ignore invalid UTF-8 (handled error)
    if let Ok(text) = std::str::from_utf8(data) {
        // Try to parse as YAML plugin manifest
        let _ = serde_yaml::from_str::<PluginManifest>(text);
        
        // Try to parse as JSON plugin manifest
        let _ = serde_json::from_str::<PluginManifest>(text);
        
        // Try to parse as YAML plugin config
        let _ = serde_yaml::from_str::<PluginConfig>(text);
        
        // Try to parse as JSON plugin config
        let _ = serde_json::from_str::<PluginConfig>(text);
    }
});
