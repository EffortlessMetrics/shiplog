//! Plugin system for loadable third-party ingest adapters.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Plugin manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub shiplog_version: String,
}

/// Plugin status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Installed,
    Enabled,
    Disabled,
}

/// Plugin manager
pub struct PluginManager {
    plugins_dir: PathBuf,
    plugins: HashMap<String, PluginManifest>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            plugins: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_status_variants() {
        assert_eq!(PluginStatus::Installed, PluginStatus::Installed);
        assert_eq!(PluginStatus::Enabled, PluginStatus::Enabled);
    }
}
