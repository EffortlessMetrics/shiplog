//! Integration tests for shiplog-report.

use chrono::{TimeZone, Utc};
use shiplog_report::*;

fn fixed_start() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0).unwrap()
}

fn fixed_end() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2025, 6, 30, 23, 59, 59).unwrap()
}

// ── Snapshot tests ──────────────────────────────────────────────────────────

#[test]
fn snapshot_empty_report_json() {
    let report = Report::new(
        "Empty Report".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Json,
    );
    // Exclude generated_at since it's dynamic
    let yaml = serde_yaml::to_string(&report.sections).unwrap();
    insta::assert_snapshot!("empty_report_sections", yaml);
}

#[test]
fn snapshot_report_with_sections() {
    let mut report = Report::new(
        "Test Report".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Markdown,
    );
    report.add_section("Summary".into(), "Total: 100 events".into(), 0);
    report.add_section("Backend".into(), "Events: 60\nCoverage: 85.0%".into(), 1);
    report.add_section("Frontend".into(), "Events: 40\nCoverage: 92.0%".into(), 2);
    report.sort_sections();

    let yaml = serde_yaml::to_string(&report.sections).unwrap();
    insta::assert_snapshot!("report_with_sections", yaml);
}

#[test]
fn snapshot_generated_report() {
    let mut generator = ReportGenerator::new();
    generator.add_workstream("api", 50, 85.0);
    generator.add_workstream("web", 30, 90.0);

    let mut report = generator.generate(
        "Sprint Report".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Json,
    );

    // Sort by title and re-assign order for deterministic snapshots
    report.sections.sort_by(|a, b| a.title.cmp(&b.title));
    for (i, section) in report.sections.iter_mut().enumerate() {
        section.order = i;
    }
    let yaml = serde_yaml::to_string(&report.sections).unwrap();
    insta::assert_snapshot!("generated_report_sections", yaml);
}

#[test]
fn snapshot_workstream_summary() {
    let ws = WorkstreamSummary {
        name: "infrastructure".into(),
        event_count: 42,
        coverage: 78.5,
    };
    insta::assert_yaml_snapshot!("workstream_summary", ws);
}

// ── Property tests ──────────────────────────────────────────────────────────

mod proptest_suite {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn sort_sections_is_stable(count in 1_usize..20) {
            let mut report = Report::new(
                "Test".into(),
                fixed_start(),
                fixed_end(),
                ReportFormat::Json,
            );
            // Add sections in reverse order
            for i in (0..count).rev() {
                report.add_section(format!("Section {i}"), format!("content {i}"), i);
            }
            report.sort_sections();

            for (idx, section) in report.sections.iter().enumerate() {
                prop_assert_eq!(section.order, idx);
            }
        }

        #[test]
        fn sort_is_idempotent(count in 1_usize..10) {
            let mut report = Report::new(
                "Test".into(),
                fixed_start(),
                fixed_end(),
                ReportFormat::Json,
            );
            for i in (0..count).rev() {
                report.add_section(format!("S{i}"), format!("c{i}"), i);
            }
            report.sort_sections();
            let first: Vec<String> = report.sections.iter().map(|s| s.title.clone()).collect();
            report.sort_sections();
            let second: Vec<String> = report.sections.iter().map(|s| s.title.clone()).collect();
            prop_assert_eq!(first, second);
        }

        #[test]
        fn generator_section_count_is_workstreams_plus_summary(ws_count in 0_usize..10) {
            let mut generator = ReportGenerator::new();
            for i in 0..ws_count {
                generator.add_workstream(&format!("ws-{i}"), i * 10, 50.0 + i as f64);
            }
            let report = generator.generate("R".into(), fixed_start(), fixed_end(), ReportFormat::Json);
            // Summary + one per workstream
            prop_assert_eq!(report.sections.len(), ws_count + 1);
        }
    }
}

// ── Edge cases ──────────────────────────────────────────────────────────────

#[test]
fn empty_report_has_no_sections() {
    let report = Report::new("E".into(), fixed_start(), fixed_end(), ReportFormat::Json);
    assert!(report.sections.is_empty());
}

#[test]
fn report_metadata_title() {
    let report = Report::new(
        "My Title".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Markdown,
    );
    assert_eq!(report.metadata.title, "My Title");
    assert!(report.metadata.author.is_none());
}

#[test]
fn report_format_equality() {
    assert_eq!(ReportFormat::Json, ReportFormat::Json);
    assert_ne!(ReportFormat::Json, ReportFormat::Markdown);
    assert_ne!(ReportFormat::Markdown, ReportFormat::Html);
}

#[test]
fn generator_no_workstreams_produces_summary_only() {
    let generator = ReportGenerator::new();
    let report = generator.generate(
        "Empty".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Json,
    );
    assert_eq!(report.sections.len(), 1);
    assert_eq!(report.sections[0].title, "Summary");
    assert!(report.sections[0].content.contains("Total workstreams: 0"));
    assert!(report.sections[0].content.contains("Total events: 0"));
    assert!(
        report.sections[0]
            .content
            .contains("Average coverage: 0.0%")
    );
}

#[test]
fn generator_duplicate_workstream_overwrites() {
    let mut generator = ReportGenerator::new();
    generator.add_workstream("api", 10, 50.0);
    generator.add_workstream("api", 20, 80.0);

    let report = generator.generate("R".into(), fixed_start(), fixed_end(), ReportFormat::Json);
    // Summary + 1 workstream (deduplicated by name)
    assert_eq!(report.sections.len(), 2);
}

#[test]
fn generator_coverage_averaging() {
    let mut generator = ReportGenerator::new();
    generator.add_workstream("a", 10, 100.0);
    generator.add_workstream("b", 10, 50.0);

    let report = generator.generate("R".into(), fixed_start(), fixed_end(), ReportFormat::Json);
    let summary = &report.sections[0];
    assert!(summary.content.contains("Average coverage: 75.0%"));
}

#[test]
fn report_default_generator() {
    let generator = ReportGenerator::default();
    let report = generator.generate("D".into(), fixed_start(), fixed_end(), ReportFormat::Html);
    assert_eq!(report.sections.len(), 1);
}

#[test]
fn workstream_summary_default() {
    let ws = WorkstreamSummary::default();
    assert!(ws.name.is_empty());
    assert_eq!(ws.event_count, 0);
    assert_eq!(ws.coverage, 0.0);
}

#[test]
fn sort_empty_sections() {
    let mut report = Report::new("E".into(), fixed_start(), fixed_end(), ReportFormat::Json);
    report.sort_sections(); // should not panic
    assert!(report.sections.is_empty());
}

#[test]
fn sections_preserve_content() {
    let mut report = Report::new("T".into(), fixed_start(), fixed_end(), ReportFormat::Json);
    report.add_section("Title".into(), "Body text\nwith newlines".into(), 0);
    assert_eq!(report.sections[0].content, "Body text\nwith newlines");
}

#[test]
fn report_serialization_round_trip() {
    let mut report = Report::new(
        "RT".into(),
        fixed_start(),
        fixed_end(),
        ReportFormat::Markdown,
    );
    report.add_section("S1".into(), "Content".into(), 0);

    let json = serde_json::to_string(&report).unwrap();
    let loaded: Report = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.metadata.title, "RT");
    assert_eq!(loaded.sections.len(), 1);
    assert_eq!(loaded.format, ReportFormat::Markdown);
}
