//! Export utilities for shiplog in various output formats.
//!
//! Provides exporters for different formats including JSON, CSV, and markdown.

use serde::{Deserialize, Serialize};
use std::fmt;

pub use shiplog_output_layout::{
    DIR_PROFILES, FILE_BUNDLE_MANIFEST_JSON, FILE_COVERAGE_MANIFEST_JSON, FILE_LEDGER_EVENTS_JSONL,
    FILE_PACKET_MD, PROFILE_INTERNAL, PROFILE_MANAGER, PROFILE_PUBLIC, RunArtifactPaths,
    zip_path_for_profile,
};

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportFormat {
    Json,
    Jsonl,
    Csv,
    Markdown,
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Json => write!(f, "json"),
            ExportFormat::Jsonl => write!(f, "jsonl"),
            ExportFormat::Csv => write!(f, "csv"),
            ExportFormat::Markdown => write!(f, "markdown"),
        }
    }
}

impl ExportFormat {
    /// Get the file extension for this format.
    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Json => "json",
            ExportFormat::Jsonl => "jsonl",
            ExportFormat::Csv => "csv",
            ExportFormat::Markdown => "md",
        }
    }
}

/// Export options for configuring output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub pretty: bool,
    pub include_metadata: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Json,
            pretty: true,
            include_metadata: true,
        }
    }
}

/// Exporter trait for different output formats.
pub trait Exporter {
    /// Export data to the specified format.
    fn export(&self, data: &ExportData, options: &ExportOptions) -> anyhow::Result<String>;
}

/// Export data container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    pub title: String,
    pub events: Vec<ExportEvent>,
}

impl ExportData {
    /// Create new export data.
    pub fn new(title: String) -> Self {
        Self {
            title,
            events: Vec::new(),
        }
    }

    /// Add an event to the export.
    pub fn add_event(&mut self, event: ExportEvent) {
        self.events.push(event);
    }
}

/// Individual event in export data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportEvent {
    pub id: String,
    pub source: String,
    pub title: String,
    pub timestamp: String,
}

/// JSON exporter implementation.
pub struct JsonExporter;

impl Exporter for JsonExporter {
    fn export(&self, data: &ExportData, options: &ExportOptions) -> anyhow::Result<String> {
        if options.pretty {
            serde_json::to_string_pretty(data)
                .map_err(|e| anyhow::anyhow!("JSON export failed: {}", e))
        } else {
            serde_json::to_string(data).map_err(|e| anyhow::anyhow!("JSON export failed: {}", e))
        }
    }
}

/// CSV exporter implementation.
pub struct CsvExporter;

impl Exporter for CsvExporter {
    fn export(&self, data: &ExportData, _options: &ExportOptions) -> anyhow::Result<String> {
        let mut output = String::new();
        // Header
        output.push_str("id,source,title,timestamp\n");
        // Events
        for event in &data.events {
            let escaped_title = event.title.replace('"', "\"\"");
            output.push_str(&format!(
                "\"{}\",\"{}\",\"{}\",\"{}\"\n",
                event.id, event.source, escaped_title, event.timestamp
            ));
        }
        Ok(output)
    }
}

/// Export helper function.
pub fn export_data(data: &ExportData, options: &ExportOptions) -> anyhow::Result<String> {
    let exporter: Box<dyn Exporter> = match options.format {
        ExportFormat::Json | ExportFormat::Jsonl => Box::new(JsonExporter {}),
        ExportFormat::Csv => Box::new(CsvExporter {}),
        ExportFormat::Markdown => Box::new(JsonExporter {}), // Fallback to JSON
    };
    exporter.export(data, options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_display() {
        assert_eq!(format!("{}", ExportFormat::Json), "json");
        assert_eq!(format!("{}", ExportFormat::Csv), "csv");
    }

    #[test]
    fn export_format_extension() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Markdown.extension(), "md");
    }

    #[test]
    fn export_options_default() {
        let options = ExportOptions::default();
        assert_eq!(options.format, ExportFormat::Json);
        assert!(options.pretty);
        assert!(options.include_metadata);
    }

    #[test]
    fn export_data_json() {
        let mut data = ExportData::new("Test Export".to_string());
        data.add_event(ExportEvent {
            id: "evt-001".to_string(),
            source: "github".to_string(),
            title: "Fix bug".to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        });

        let options = ExportOptions::default();
        let result = export_data(&data, &options);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("evt-001"));
    }

    #[test]
    fn export_data_csv() {
        let mut data = ExportData::new("Test Export".to_string());
        data.add_event(ExportEvent {
            id: "evt-001".to_string(),
            source: "github".to_string(),
            title: "Fix bug".to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        });

        let options = ExportOptions {
            format: ExportFormat::Csv,
            pretty: false,
            include_metadata: false,
        };
        let result = export_data(&data, &options);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("id,source,title,timestamp"));
        assert!(output.contains("evt-001"));
    }

    #[test]
    fn csv_exporter_escapes_quotes() {
        let mut data = ExportData::new("Test".to_string());
        data.add_event(ExportEvent {
            id: "evt-001".to_string(),
            source: "github".to_string(),
            title: "Fix \"critical\" bug".to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        });

        let exporter = CsvExporter {};
        let options = ExportOptions::default();
        let result = exporter.export(&data, &options);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("\"\"critical\"\""));
    }
}
