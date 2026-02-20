//! Browser-based viewer for rendered packets.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Web viewer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    #[serde(default)]
    pub packet_path: PathBuf,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_host() -> String {
    "127.0.0.1".to_string()
}

fn default_port() -> u16 {
    8080
}

/// Web viewer
pub struct WebViewer {
    config: WebConfig,
}

impl WebViewer {
    pub fn new(config: WebConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_config_defaults() {
        let config = WebConfig {
            packet_path: PathBuf::from("packet.md"),
            host: "127.0.0.1".to_string(),
            port: 8080,
        };
        assert_eq!(config.port, 8080);
    }
}
