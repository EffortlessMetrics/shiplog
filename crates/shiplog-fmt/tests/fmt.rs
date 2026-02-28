//! Integration tests for shiplog-fmt.

use chrono::{Duration, TimeZone, Utc};
use proptest::prelude::*;
use shiplog_fmt::*;

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_format_timestamp() {
    let dt = Utc.with_ymd_and_hms(2024, 6, 15, 14, 30, 45).unwrap();
    insta::assert_snapshot!(format_timestamp(&dt), @"2024-06-15 14:30:45 UTC");
}

#[test]
fn snapshot_format_duration_matrix() {
    let cases = [0, 1, 59, 60, 61, 3599, 3600, 3661, 86400];
    let formatted: Vec<String> = cases
        .iter()
        .map(|&s| format!("{:>6}s => {}", s, format_duration(Duration::seconds(s))))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

#[test]
fn snapshot_format_size_matrix() {
    let cases: Vec<u64> = vec![0, 1, 512, 1023, 1024, 1536, 1048576, 1073741824, u64::MAX];
    let formatted: Vec<String> = cases
        .iter()
        .map(|&b| format!("{:>20} => {}", b, format_size(b)))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

#[test]
fn snapshot_format_number_matrix() {
    let cases: Vec<u64> = vec![
        0,
        1,
        42,
        999,
        1000,
        9999,
        10000,
        999_999,
        1_000_000,
        1_000_000_000,
    ];
    let formatted: Vec<String> = cases
        .iter()
        .map(|&n| format!("{:>13} => {}", n, format_number(n)))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

// ── Known-answer tests ──────────────────────────────────────────

#[test]
fn format_size_known_answers() {
    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(1), "1 B");
    assert_eq!(format_size(1023), "1023 B");
    assert_eq!(format_size(1024), "1.00 KB");
    assert_eq!(format_size(1536), "1.50 KB");
    assert_eq!(format_size(1048576), "1.00 MB");
    assert_eq!(format_size(1073741824), "1.00 GB");
}

#[test]
fn format_number_known_answers() {
    assert_eq!(format_number(0), "0");
    assert_eq!(format_number(1), "1");
    assert_eq!(format_number(999), "999");
    assert_eq!(format_number(1_000), "1,000");
    assert_eq!(format_number(1_234_567), "1,234,567");
    assert_eq!(format_number(1_000_000_000), "1,000,000,000");
}

#[test]
fn format_duration_known_answers() {
    assert_eq!(format_duration(Duration::seconds(0)), "0s");
    assert_eq!(format_duration(Duration::seconds(59)), "59s");
    assert_eq!(format_duration(Duration::seconds(60)), "1m");
    assert_eq!(format_duration(Duration::seconds(90)), "1m 30s");
    assert_eq!(format_duration(Duration::seconds(3600)), "1h");
    assert_eq!(format_duration(Duration::seconds(7200)), "2h");
    assert_eq!(format_duration(Duration::seconds(3661)), "1h 1m");
}

// ── Edge cases ──────────────────────────────────────────────────

#[test]
fn pad_empty_string() {
    assert_eq!(pad("", 5), "     ");
}

#[test]
fn pad_width_zero() {
    assert_eq!(pad("hello", 0), "hello");
}

#[test]
fn pad_exact_width() {
    assert_eq!(pad("abc", 3), "abc");
}

#[test]
fn truncate_empty_string() {
    assert_eq!(truncate("", 10), "");
}

#[test]
fn truncate_exact_width() {
    assert_eq!(truncate("abc", 3), "abc");
}

#[test]
fn truncate_at_minimum_for_ellipsis() {
    // max_width=3 means we take s[..0] + "..." => "..."
    assert_eq!(truncate("abcdef", 3), "...");
}

#[test]
fn indent_empty_string() {
    // "".lines() produces no lines, so result is empty
    assert_eq!(indent("", 4), "");
}

#[test]
fn indent_multiline() {
    let result = indent("a\nb\nc", 2);
    assert_eq!(result, "  a\n  b\n  c");
}

#[test]
fn indent_zero_spaces() {
    assert_eq!(indent("hello", 0), "hello");
}

#[test]
fn format_relative_time_singular_forms() {
    let one_hour = Utc::now() - Duration::hours(1);
    assert_eq!(format_relative_time(&one_hour), "1 hour ago");

    let one_day = Utc::now() - Duration::days(1);
    assert_eq!(format_relative_time(&one_day), "1 day ago");

    let one_minute = Utc::now() - Duration::minutes(1);
    assert_eq!(format_relative_time(&one_minute), "1 minute ago");
}

#[test]
fn format_relative_time_years() {
    let years_ago = Utc::now() - Duration::days(400);
    let result = format_relative_time(&years_ago);
    assert!(result.contains("year"));
}

#[test]
fn format_relative_time_months() {
    let months_ago = Utc::now() - Duration::days(60);
    let result = format_relative_time(&months_ago);
    assert!(result.contains("month"));
}

#[test]
fn format_config_default_values() {
    let config = FormatConfig::default();
    assert_eq!(config.output_format, OutputFormat::Plain);
    assert!(config.show_timestamps);
    assert!(!config.show_metadata);
    assert_eq!(config.indent_size, 2);
}

#[test]
fn output_format_default_is_plain() {
    assert_eq!(OutputFormat::default(), OutputFormat::Plain);
}

// ── Unicode edge cases ──────────────────────────────────────────

#[test]
fn pad_unicode() {
    // Note: pad uses byte length not char count, so behavior with multi-byte
    // chars may differ. This tests current behavior.
    let result = pad("日本", 10);
    assert!(result.len() >= 10 || result.starts_with("日本"));
}

#[test]
fn indent_unicode() {
    let result = indent("こんにちは\n世界", 2);
    assert!(result.contains("  こんにちは"));
    assert!(result.contains("  世界"));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_format_number_contains_only_digits_and_commas(n: u64) {
        let formatted = format_number(n);
        prop_assert!(formatted.chars().all(|c| c.is_ascii_digit() || c == ','));
    }

    #[test]
    fn prop_format_number_no_leading_comma(n: u64) {
        let formatted = format_number(n);
        prop_assert!(!formatted.starts_with(','));
    }

    #[test]
    fn prop_format_number_digits_preserved(n: u64) {
        let formatted = format_number(n);
        let stripped: String = formatted.chars().filter(|c| *c != ',').collect();
        prop_assert_eq!(stripped, n.to_string());
    }

    #[test]
    fn prop_pad_at_least_original_length(s in "[a-zA-Z0-9]{0,20}", width in 0usize..50) {
        let padded = pad(&s, width);
        prop_assert!(padded.len() >= s.len());
    }

    #[test]
    fn prop_pad_preserves_prefix(s in "[a-zA-Z0-9]{0,20}", width in 0usize..50) {
        let padded = pad(&s, width);
        prop_assert!(padded.starts_with(&s));
    }

    #[test]
    fn prop_truncate_respects_max(s in "[a-zA-Z0-9]{0,100}", max in 3usize..50) {
        let truncated = truncate(&s, max);
        prop_assert!(truncated.len() <= max);
    }

    #[test]
    fn prop_indent_adds_prefix_to_every_line(
        text in "[a-zA-Z0-9 ]{1,20}(\n[a-zA-Z0-9 ]{1,20}){0,5}",
        spaces in 0usize..10
    ) {
        let indented = indent(&text, spaces);
        let prefix = " ".repeat(spaces);
        for line in indented.lines() {
            prop_assert!(line.starts_with(&prefix));
        }
    }

    #[test]
    fn prop_format_size_never_empty(bytes: u64) {
        let formatted = format_size(bytes);
        prop_assert!(!formatted.is_empty());
    }

    #[test]
    fn prop_format_duration_never_empty(secs in 0i64..1_000_000) {
        let formatted = format_duration(Duration::seconds(secs));
        prop_assert!(!formatted.is_empty());
    }
}
