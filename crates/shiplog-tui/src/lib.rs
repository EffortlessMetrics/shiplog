//! Interactive terminal UI for curating workstreams.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// TUI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuiConfig {
    #[serde(default)]
    pub workstreams_path: PathBuf,
    #[serde(default = "default_true")]
    pub show_receipts: bool,
}

fn default_true() -> bool {
    true
}

/// TUI mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiMode {
    List,
    EditTitle,
    EditSummary,
    EditReceipts,
}

/// TUI editor
pub struct TuiEditor {
    #[allow(dead_code)]
    config: TuiConfig,
    #[allow(dead_code)]
    mode: TuiMode,
}

impl TuiEditor {
    pub fn new(config: TuiConfig) -> Self {
        Self {
            config,
            mode: TuiMode::List,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tui_mode_list() {
        assert_eq!(TuiMode::List, TuiMode::List);
    }
}
