//! Team aggregation mode for generating team-level shipping summaries.
//!
//! This crate provides minimal functionality for aggregating team member ledgers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Team configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub members: Vec<String>,
    #[serde(default)]
    pub aliases: HashMap<String, String>,
    #[serde(default)]
    pub sections: Vec<String>,
    #[serde(default)]
    pub template: Option<PathBuf>,
}

/// Team aggregator
pub struct TeamAggregator {
    #[allow(dead_code)]
    config: TeamConfig,
}

impl TeamAggregator {
    pub fn new(config: TeamConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_config_default() {
        let config = TeamConfig {
            members: vec!["alice".to_string()],
            aliases: HashMap::new(),
            sections: vec![],
            template: None,
        };
        assert_eq!(config.members.len(), 1);
    }
}
