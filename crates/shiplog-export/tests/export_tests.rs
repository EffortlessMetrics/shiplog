use shiplog_export::*;
use std::path::Path;

// Test re-exports from shiplog-output-layout
#[test]
fn reexported_constants() {
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
fn reexported_run_artifact_paths() {
    let paths = RunArtifactPaths::new("out/run");
    assert!(paths.packet_md().ends_with("packet.md"));
}

#[test]
fn reexported_zip_path() {
    let p = zip_path_for_profile(Path::new("out/run"), PROFILE_INTERNAL);
    assert!(p.to_string_lossy().ends_with(".zip"));
}

#[test]
fn export_format_display() {
    assert_eq!(ExportFormat::Json.to_string(), "json");
    assert_eq!(ExportFormat::Jsonl.to_string(), "jsonl");
    assert_eq!(ExportFormat::Csv.to_string(), "csv");
    assert_eq!(ExportFormat::Markdown.to_string(), "markdown");
}

#[test]
fn export_format_extension() {
    assert_eq!(ExportFormat::Json.extension(), "json");
    assert_eq!(ExportFormat::Jsonl.extension(), "jsonl");
    assert_eq!(ExportFormat::Csv.extension(), "csv");
    assert_eq!(ExportFormat::Markdown.extension(), "md");
}

#[test]
fn export_format_equality() {
    assert_eq!(ExportFormat::Json, ExportFormat::Json);
    assert_ne!(ExportFormat::Json, ExportFormat::Csv);
}
