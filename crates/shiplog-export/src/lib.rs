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

/// JSONL (newline-delimited JSON) exporter implementation.
pub struct JsonlExporter;

impl Exporter for JsonlExporter {
    fn export(&self, data: &ExportData, _options: &ExportOptions) -> anyhow::Result<String> {
        let lines: Result<Vec<String>, _> = data.events.iter().map(serde_json::to_string).collect();
        let lines = lines.map_err(|e| anyhow::anyhow!("JSONL export failed: {}", e))?;
        Ok(lines.join("\n"))
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
        ExportFormat::Json => Box::new(JsonExporter {}),
        ExportFormat::Jsonl => Box::new(JsonlExporter {}),
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

    #[test]
    fn export_data_jsonl() {
        let mut data = ExportData::new("Test Export".to_string());
        data.add_event(ExportEvent {
            id: "evt-001".to_string(),
            source: "github".to_string(),
            title: "Fix bug".to_string(),
            timestamp: "2024-01-15T10:00:00Z".to_string(),
        });
        data.add_event(ExportEvent {
            id: "evt-002".to_string(),
            source: "manual".to_string(),
            title: "Add feature".to_string(),
            timestamp: "2024-01-16T10:00:00Z".to_string(),
        });

        let options = ExportOptions {
            format: ExportFormat::Jsonl,
            pretty: false,
            include_metadata: false,
        };
        let result = export_data(&data, &options);
        assert!(result.is_ok());
        let output = result.unwrap();
        let lines: Vec<&str> = output.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("evt-001"));
        assert!(lines[1].contains("evt-002"));
        // Each line should be valid JSON
        for line in &lines {
            serde_json::from_str::<ExportEvent>(line).unwrap();
        }
    }

    #[test]
    fn export_data_jsonl_empty() {
        let data = ExportData::new("Empty".to_string());
        let options = ExportOptions {
            format: ExportFormat::Jsonl,
            pretty: false,
            include_metadata: false,
        };
        let result = export_data(&data, &options);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }

    // --- Edge case tests ---

    #[test]
    fn export_format_display_all_variants() {
        assert_eq!(format!("{}", ExportFormat::Jsonl), "jsonl");
        assert_eq!(format!("{}", ExportFormat::Markdown), "markdown");
    }

    #[test]
    fn export_format_extension_jsonl() {
        assert_eq!(ExportFormat::Jsonl.extension(), "jsonl");
    }

    #[test]
    fn export_format_eq_and_copy() {
        let a = ExportFormat::Json;
        let b = a; // Copy
        assert_eq!(a, b);
        assert_ne!(ExportFormat::Json, ExportFormat::Csv);
    }

    #[test]
    fn export_data_new_starts_empty() {
        let data = ExportData::new("My Title".to_string());
        assert_eq!(data.title, "My Title");
        assert!(data.events.is_empty());
    }

    #[test]
    fn export_data_add_event_accumulates() {
        let mut data = ExportData::new("T".to_string());
        for i in 0..5 {
            data.add_event(ExportEvent {
                id: format!("e{}", i),
                source: "github".to_string(),
                title: format!("Event {}", i),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            });
        }
        assert_eq!(data.events.len(), 5);
    }

    #[test]
    fn json_exporter_pretty_vs_compact() {
        let mut data = ExportData::new("T".to_string());
        data.add_event(ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "X".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });

        let pretty_opts = ExportOptions {
            format: ExportFormat::Json,
            pretty: true,
            include_metadata: false,
        };
        let compact_opts = ExportOptions {
            format: ExportFormat::Json,
            pretty: false,
            include_metadata: false,
        };

        let pretty = export_data(&data, &pretty_opts).unwrap();
        let compact = export_data(&data, &compact_opts).unwrap();
        assert!(
            pretty.contains('\n'),
            "pretty output should contain newlines"
        );
        assert!(
            !compact.contains('\n'),
            "compact output should not contain newlines"
        );
        // Both should parse to the same value
        let p: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        let c: serde_json::Value = serde_json::from_str(&compact).unwrap();
        assert_eq!(p, c);
    }

    #[test]
    fn json_export_empty_events() {
        let data = ExportData::new("Empty".to_string());
        let options = ExportOptions::default();
        let result = export_data(&data, &options).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert!(parsed["events"].as_array().unwrap().is_empty());
    }

    #[test]
    fn csv_header_always_present() {
        let data = ExportData::new("T".to_string());
        let exporter = CsvExporter;
        let options = ExportOptions::default();
        let result = exporter.export(&data, &options).unwrap();
        assert!(result.starts_with("id,source,title,timestamp\n"));
    }

    #[test]
    fn csv_empty_events_only_header() {
        let data = ExportData::new("T".to_string());
        let exporter = CsvExporter;
        let options = ExportOptions::default();
        let result = exporter.export(&data, &options).unwrap();
        assert_eq!(result, "id,source,title,timestamp\n");
    }

    #[test]
    fn csv_multiple_events_correct_line_count() {
        let mut data = ExportData::new("T".to_string());
        for i in 0..3 {
            data.add_event(ExportEvent {
                id: format!("e{}", i),
                source: "github".to_string(),
                title: format!("Title {}", i),
                timestamp: "2024-01-01T00:00:00Z".to_string(),
            });
        }
        let exporter = CsvExporter;
        let options = ExportOptions::default();
        let result = exporter.export(&data, &options).unwrap();
        // Header + 3 data lines, each terminated with \n
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 events
    }

    #[test]
    fn csv_event_with_commas_in_title() {
        let mut data = ExportData::new("T".to_string());
        data.add_event(ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "Fix bug, improve perf, add tests".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });
        let exporter = CsvExporter;
        let options = ExportOptions::default();
        let result = exporter.export(&data, &options).unwrap();
        // Title is quoted, so commas inside should be safe
        assert!(result.contains("\"Fix bug, improve perf, add tests\""));
    }

    #[test]
    fn jsonl_single_event_no_trailing_newline() {
        let mut data = ExportData::new("T".to_string());
        data.add_event(ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "X".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });
        let options = ExportOptions {
            format: ExportFormat::Jsonl,
            pretty: false,
            include_metadata: false,
        };
        let result = export_data(&data, &options).unwrap();
        assert_eq!(result.lines().count(), 1);
        assert!(!result.ends_with('\n'));
    }

    #[test]
    fn markdown_format_falls_back_to_json() {
        let mut data = ExportData::new("T".to_string());
        data.add_event(ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "X".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });
        let md_opts = ExportOptions {
            format: ExportFormat::Markdown,
            pretty: true,
            include_metadata: false,
        };
        let json_opts = ExportOptions {
            format: ExportFormat::Json,
            pretty: true,
            include_metadata: false,
        };
        let md_result = export_data(&data, &md_opts).unwrap();
        let json_result = export_data(&data, &json_opts).unwrap();
        assert_eq!(md_result, json_result);
    }

    #[test]
    fn export_event_serde_roundtrip() {
        let event = ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "Test \"special\" chars & more".to_string(),
            timestamp: "2024-06-15T12:30:00Z".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let de: ExportEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(de.id, event.id);
        assert_eq!(de.title, event.title);
    }

    #[test]
    fn export_data_serde_roundtrip() {
        let mut data = ExportData::new("Roundtrip".to_string());
        data.add_event(ExportEvent {
            id: "e1".to_string(),
            source: "github".to_string(),
            title: "T".to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        });
        let json = serde_json::to_string(&data).unwrap();
        let de: ExportData = serde_json::from_str(&json).unwrap();
        assert_eq!(de.title, data.title);
        assert_eq!(de.events.len(), 1);
    }

    #[test]
    fn export_options_serde_roundtrip() {
        let opts = ExportOptions {
            format: ExportFormat::Csv,
            pretty: false,
            include_metadata: true,
        };
        let json = serde_json::to_string(&opts).unwrap();
        let de: ExportOptions = serde_json::from_str(&json).unwrap();
        assert_eq!(de.format, ExportFormat::Csv);
        assert!(!de.pretty);
        assert!(de.include_metadata);
    }

    #[test]
    fn re_exported_layout_constants_are_available() {
        // Verify re-exports from shiplog-output-layout work
        assert_eq!(FILE_PACKET_MD, "packet.md");
        assert_eq!(FILE_LEDGER_EVENTS_JSONL, "ledger.events.jsonl");
        assert_eq!(FILE_COVERAGE_MANIFEST_JSON, "coverage.manifest.json");
        assert_eq!(FILE_BUNDLE_MANIFEST_JSON, "bundle.manifest.json");
        assert_eq!(DIR_PROFILES, "profiles");
        assert_eq!(PROFILE_INTERNAL, "internal");
        assert_eq!(PROFILE_MANAGER, "manager");
        assert_eq!(PROFILE_PUBLIC, "public");
    }

    #[test]
    fn re_exported_run_artifact_paths_works() {
        let paths = RunArtifactPaths::new("/tmp/run");
        assert!(paths.packet_md().ends_with("packet.md"));
    }

    #[test]
    fn re_exported_zip_path_for_profile_works() {
        let p = zip_path_for_profile(std::path::Path::new("/tmp/run"), "public");
        assert!(p.to_string_lossy().contains("public"));
    }

    // --- Property tests ---

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_export_event() -> impl Strategy<Value = ExportEvent> {
            (
                "[a-z0-9\\-]{1,20}",
                prop_oneof!["github", "manual", "jira", "linear"],
                ".*",
                "20[0-9]{2}-[01][0-9]-[0-3][0-9]T[0-2][0-9]:[0-5][0-9]:[0-5][0-9]Z",
            )
                .prop_map(|(id, source, title, timestamp)| ExportEvent {
                    id,
                    source,
                    title,
                    timestamp,
                })
        }

        proptest! {
            #[test]
            fn json_export_always_produces_valid_json(
                events in proptest::collection::vec(arb_export_event(), 0..10),
                pretty in proptest::bool::ANY,
            ) {
                let mut data = ExportData::new("PropTest".to_string());
                for e in events {
                    data.add_event(e);
                }
                let opts = ExportOptions { format: ExportFormat::Json, pretty, include_metadata: true };
                let result = export_data(&data, &opts).unwrap();
                let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
                prop_assert!(parsed.is_object());
            }

            #[test]
            fn jsonl_lines_count_equals_event_count(
                events in proptest::collection::vec(arb_export_event(), 1..20),
            ) {
                let n = events.len();
                let mut data = ExportData::new("T".to_string());
                for e in events {
                    data.add_event(e);
                }
                let opts = ExportOptions { format: ExportFormat::Jsonl, pretty: false, include_metadata: false };
                let result = export_data(&data, &opts).unwrap();
                prop_assert_eq!(result.lines().count(), n);
            }

            #[test]
            fn jsonl_each_line_is_valid_json(
                events in proptest::collection::vec(arb_export_event(), 1..10),
            ) {
                let mut data = ExportData::new("T".to_string());
                for e in events {
                    data.add_event(e);
                }
                let opts = ExportOptions { format: ExportFormat::Jsonl, pretty: false, include_metadata: false };
                let result = export_data(&data, &opts).unwrap();
                for line in result.lines() {
                    let parsed: Result<ExportEvent, _> = serde_json::from_str(line);
                    prop_assert!(parsed.is_ok(), "line is not valid JSON: {}", line);
                }
            }

            #[test]
            fn csv_line_count_is_header_plus_events(
                events in proptest::collection::vec(arb_export_event(), 0..15),
            ) {
                let n = events.len();
                let mut data = ExportData::new("T".to_string());
                for e in events {
                    data.add_event(e);
                }
                let exporter = CsvExporter;
                let opts = ExportOptions::default();
                let result = exporter.export(&data, &opts).unwrap();
                let line_count = result.lines().count();
                // header + n events
                prop_assert_eq!(line_count, 1 + n);
            }

            #[test]
            fn export_event_serde_roundtrip_prop(event in arb_export_event()) {
                let json = serde_json::to_string(&event).unwrap();
                let de: ExportEvent = serde_json::from_str(&json).unwrap();
                prop_assert_eq!(de.id, event.id);
                prop_assert_eq!(de.source, event.source);
                prop_assert_eq!(de.title, event.title);
                prop_assert_eq!(de.timestamp, event.timestamp);
            }

            #[test]
            fn export_format_display_matches_extension_logic(
                fmt in prop_oneof![
                    Just(ExportFormat::Json),
                    Just(ExportFormat::Jsonl),
                    Just(ExportFormat::Csv),
                    Just(ExportFormat::Markdown),
                ]
            ) {
                let display = format!("{}", fmt);
                let ext = fmt.extension();
                // Display and extension are both non-empty
                prop_assert!(!display.is_empty());
                prop_assert!(!ext.is_empty());
            }
        }
    }
}
