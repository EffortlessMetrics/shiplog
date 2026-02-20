//! Configuration management and loading for shiplog.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration format types supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Json,
    Yaml,
}

impl Default for ConfigFormat {
    fn default() -> Self {
        Self::Yaml
    }
}

/// Main shiplog configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShiplogConfig {
    /// Output directory for packets
    #[serde(default = "default_output_dir")]
    pub output_dir: PathBuf,
    
    /// Cache directory for data
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,
    
    /// Enable incremental collection
    #[serde(default = "default_true")]
    pub incremental: bool,
    
    /// Verbose logging
    #[serde(default)]
    pub verbose: bool,
    
    /// Workstream configurations
    #[serde(default)]
    pub workstreams: Vec<WorkstreamConfig>,
}

fn default_output_dir() -> PathBuf {
    PathBuf::from("./packets")
}

fn default_cache_dir() -> PathBuf {
    PathBuf::from("./.shiplog-cache")
}

fn default_true() -> bool {
    true
}

/// Workstream-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkstreamConfig {
    /// Workstream name
    pub name: String,
    
    /// Source type (github, jira, gitlab, etc.)
    pub source: String,
    
    /// Source-specific configuration
    #[serde(default)]
    pub config: serde_json::Value,
    
    /// Enable this workstream
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for ShiplogConfig {
    fn default() -> Self {
        Self {
            output_dir: default_output_dir(),
            cache_dir: default_cache_dir(),
            incremental: true,
            verbose: false,
            workstreams: Vec::new(),
        }
    }
}

/// Load configuration from a file
pub fn load_config<P: Into<PathBuf>>(path: P) -> anyhow::Result<ShiplogConfig> {
    let path = path.into();
    let contents = std::fs::read_to_string(&path)?;
    
    // Detect format from extension
    let format = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => ConfigFormat::Json,
        Some("yaml") | Some("yml") => ConfigFormat::Yaml,
        _ => ConfigFormat::default(),
    };
    
    match format {
        ConfigFormat::Json => {
            serde_json::from_str(&contents).map_err(|e| anyhow::anyhow!("Failed to parse JSON config: {}", e))
        }
        ConfigFormat::Yaml => {
            serde_yaml::from_str(&contents).map_err(|e| anyhow::anyhow!("Failed to parse YAML config: {}", e))
        }
    }
}

/// Save configuration to a file
pub fn save_config<P: Into<PathBuf>>(config: &ShiplogConfig, path: P) -> anyhow::Result<()> {
    let path = path.into();
    let contents = match path.extension().and_then(|e| e.to_str()) {
        Some("json") => serde_json::to_string_pretty(config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize JSON config: {}", e))?,
        _ => serde_yaml::to_string(config)
            .map_err(|e| anyhow::anyhow!("Failed to serialize YAML config: {}", e))?,
    };
    
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn config_default_values() {
        let config = ShiplogConfig::default();
        assert!(config.incremental);
        assert!(!config.verbose);
        assert_eq!(config.output_dir, PathBuf::from("./packets"));
        assert_eq!(config.cache_dir, PathBuf::from("./.shiplog-cache"));
    }

    #[test]
    fn config_serialize_yaml() {
        let config = ShiplogConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("output_dir"));
    }

    #[test]
    fn config_serialize_json() {
        let config = ShiplogConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("output_dir"));
    }

    #[test]
    fn load_save_yaml_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        let config = ShiplogConfig {
            output_dir: PathBuf::from("/tmp/output"),
            cache_dir: PathBuf::from("/tmp/cache"),
            incremental: false,
            verbose: true,
            workstreams: vec![
                WorkstreamConfig {
                    name: "test".to_string(),
                    source: "github".to_string(),
                    config: serde_json::json!({"token": "test"}),
                    enabled: true,
                },
            ],
        };
        
        save_config(&config, &config_path).unwrap();
        let loaded = load_config(&config_path).unwrap();
        
        assert_eq!(loaded.output_dir, config.output_dir);
        assert_eq!(loaded.incremental, false);
        assert_eq!(loaded.verbose, true);
        assert_eq!(loaded.workstreams.len(), 1);
    }

    #[test]
    fn load_save_json_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.json");
        
        let config = ShiplogConfig::default();
        
        save_config(&config, &config_path).unwrap();
        let loaded = load_config(&config_path).unwrap();
        
        assert_eq!(loaded.output_dir, config.output_dir);
    }
}
