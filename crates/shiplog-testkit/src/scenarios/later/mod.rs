//! BDD scenarios for Later (Exploratory) features
//!
//! This module implements BDD scenarios for the Later (Exploratory) features:
//! 1. Team Aggregation Mode
//! 2. Continuous/Cron Mode
//! 3. TUI Workstream Editor
//! 4. Web Viewer
//! 5. Plugin System
//!
//! These scenarios follow the Given/When/Then pattern and can be used
//! to verify the behavior of these features.

pub mod team_aggregation;
pub mod cron_mode;
pub mod tui_editor;
pub mod web_viewer;
pub mod plugin_system;

// Re-export all scenarios for convenience
pub use team_aggregation::*;
pub use cron_mode::*;
pub use tui_editor::*;
pub use web_viewer::*;
pub use plugin_system::*;
