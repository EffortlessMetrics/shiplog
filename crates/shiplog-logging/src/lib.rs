//! Logging configuration and utilities for shiplog.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Log level for filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl LogLevel {
    /// Check if this level should log messages at the given level
    pub fn should_log(&self, level: LogLevel) -> bool {
        let self_level = match self {
            LogLevel::Error => 0,
            LogLevel::Warn => 1,
            LogLevel::Info => 2,
            LogLevel::Debug => 3,
            LogLevel::Trace => 4,
        };
        
        let target_level = match level {
            LogLevel::Error => 0,
            LogLevel::Warn => 1,
            LogLevel::Info => 2,
            LogLevel::Debug => 3,
            LogLevel::Trace => 4,
        };
        
        self_level >= target_level
    }
}

/// Log output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Plain,
    Json,
    Compact,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Plain
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Minimum log level to output
    pub level: LogLevel,
    /// Output format
    pub format: LogFormat,
    /// Enable timestamps
    #[serde(default = "default_true")]
    pub timestamps: bool,
    /// Enable colors (for terminal output)
    #[serde(default = "default_true")]
    pub colors: bool,
    /// Component-specific log levels
    #[serde(default)]
    pub component_levels: HashMap<String, LogLevel>,
}

fn default_true() -> bool {
    true
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            format: LogFormat::Plain,
            timestamps: true,
            colors: true,
            component_levels: HashMap::new(),
        }
    }
}

impl LoggingConfig {
    /// Create a new logging config with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the log level
    pub fn with_level(mut self, level: LogLevel) -> Self {
        self.level = level;
        self
    }

    /// Set the log format
    pub fn with_format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    /// Set a component-specific log level
    pub fn with_component_level(mut self, component: impl Into<String>, level: LogLevel) -> Self {
        self.component_levels.insert(component.into(), level);
        self
    }

    /// Get the effective log level for a component
    pub fn effective_level(&self, component: Option<&str>) -> LogLevel {
        if let Some(comp) = component {
            if let Some(&level) = self.component_levels.get(comp) {
                return level;
            }
        }
        self.level
    }

    /// Check if a message at the given level should be logged
    pub fn should_log(&self, level: LogLevel, component: Option<&str>) -> bool {
        let effective = self.effective_level(component);
        effective.should_log(level)
    }
}

/// A log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: LogLevel,
    pub component: Option<String>,
    pub message: String,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            component: None,
            message: message.into(),
        }
    }

    /// Create a log entry with a component
    pub fn with_component(level: LogLevel, component: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level,
            component: Some(component.into()),
            message: message.into(),
        }
    }
}

/// Log collector for capturing log entries
#[derive(Debug, Default)]
pub struct LogCollector {
    entries: Vec<LogEntry>,
}

impl LogCollector {
    /// Create a new log collector
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Add a log entry
    pub fn push(&mut self, entry: LogEntry) {
        self.entries.push(entry);
    }

    /// Get all collected entries
    pub fn entries(&self) -> &[LogEntry] {
        &self.entries
    }

    /// Get entries matching a level
    pub fn filter_by_level(&self, level: LogLevel) -> Vec<&LogEntry> {
        self.entries
            .iter()
            .filter(|e| e.level == level)
            .collect()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Info.should_log(LogLevel::Info));
        assert!(LogLevel::Info.should_log(LogLevel::Warn));
        assert!(LogLevel::Info.should_log(LogLevel::Error));
        assert!(!LogLevel::Info.should_log(LogLevel::Debug));
    }

    #[test]
    fn logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, LogLevel::Info);
        assert_eq!(config.format, LogFormat::Plain);
    }

    #[test]
    fn logging_config_component_levels() {
        let config = LoggingConfig::new()
            .with_level(LogLevel::Warn)
            .with_component_level("network", LogLevel::Debug);
        
        // Default level is Warn
        assert!(!config.should_log(LogLevel::Info, None));
        
        // Network component has Debug level
        assert!(config.should_log(LogLevel::Debug, Some("network")));
    }

    #[test]
    fn log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, "Test message");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "Test message");
        assert!(entry.timestamp.len() > 0);
    }

    #[test]
    fn log_entry_with_component() {
        let entry = LogEntry::with_component(LogLevel::Debug, "engine", "Processing");
        assert_eq!(entry.component, Some("engine".to_string()));
    }

    #[test]
    fn log_collector() {
        let mut collector = LogCollector::new();
        
        collector.push(LogEntry::new(LogLevel::Info, "Info message"));
        collector.push(LogEntry::new(LogLevel::Error, "Error message"));
        collector.push(LogEntry::new(LogLevel::Debug, "Debug message"));
        
        assert_eq!(collector.entries().len(), 3);
        
        let errors = collector.filter_by_level(LogLevel::Error);
        assert_eq!(errors.len(), 1);
    }
}
