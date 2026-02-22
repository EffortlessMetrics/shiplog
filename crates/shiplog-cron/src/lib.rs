//! Continuous/cron mode for scheduled shiplog collection.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Cron configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronConfig {
    pub schedule: String,
    #[serde(default = "default_true")]
    pub incremental: bool,
    #[serde(default)]
    pub output_dir: PathBuf,
}

fn default_true() -> bool {
    true
}

/// Cron scheduler
pub struct CronScheduler {
    #[allow(dead_code)]
    config: CronConfig,
}

impl CronScheduler {
    pub fn new(config: CronConfig) -> Self {
        Self { config }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_config_default() {
        let config = CronConfig {
            schedule: "0 0 * * 0".to_string(),
            incremental: true,
            output_dir: PathBuf::from("/tmp"),
        };
        assert!(config.incremental);
    }
}
