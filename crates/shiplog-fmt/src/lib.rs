//! Formatting utilities for shiplog.
//!
//! This crate provides formatting utilities for displaying shiplog data
//! in various formats.

use chrono::{DateTime, Utc, Duration};

/// Output format for displaying data
#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Plain,
    Compact,
    Detailed,
    Json,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Plain
    }
}

/// Configuration for formatting
#[derive(Debug, Clone)]
pub struct FormatConfig {
    pub output_format: OutputFormat,
    pub show_timestamps: bool,
    pub show_metadata: bool,
    pub indent_size: usize,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            output_format: OutputFormat::Plain,
            show_timestamps: true,
            show_metadata: false,
            indent_size: 2,
        }
    }
}

/// Format a timestamp for display
pub fn format_timestamp(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Format a relative time (e.g., "2 hours ago")
pub fn format_relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let diff = now.signed_duration_since(*dt);
    
    if diff.num_days() > 365 {
        let years = diff.num_days() / 365;
        return format!("{} year{} ago", years, if years > 1 { "s" } else { "" });
    }
    if diff.num_days() > 30 {
        let months = diff.num_days() / 30;
        return format!("{} month{} ago", months, if months > 1 { "s" } else { "" });
    }
    if diff.num_days() > 0 {
        return format!("{} day{} ago", diff.num_days(), if diff.num_days() > 1 { "s" } else { "" });
    }
    if diff.num_hours() > 0 {
        return format!("{} hour{} ago", diff.num_hours(), if diff.num_hours() > 1 { "s" } else { "" });
    }
    if diff.num_minutes() > 0 {
        return format!("{} minute{} ago", diff.num_minutes(), if diff.num_minutes() > 1 { "s" } else { "" });
    }
    "just now".to_string()
}

/// Format a duration in human-readable form
pub fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();
    
    if total_seconds < 60 {
        return format!("{}s", total_seconds);
    }
    if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        if seconds > 0 {
            return format!("{}m {}s", minutes, seconds);
        }
        return format!("{}m", minutes);
    }
    
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    if minutes > 0 {
        return format!("{}h {}m", hours, minutes);
    }
    format!("{}h", hours)
}

/// Format a size in bytes to human-readable form
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    
    if bytes >= GB {
        return format!("{:.2} GB", bytes as f64 / GB as f64);
    }
    if bytes >= MB {
        return format!("{:.2} MB", bytes as f64 / MB as f64);
    }
    if bytes >= KB {
        return format!("{:.2} KB", bytes as f64 / KB as f64);
    }
    format!("{} B", bytes)
}

/// Format a number with thousands separators
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::new();
    
    for (i, c) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(*c);
    }
    result
}

/// Pad a string to a fixed width
pub fn pad(s: &str, width: usize) -> String {
    let len = s.len();
    if len >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - len))
    }
}

/// Truncate a string to a maximum width
pub fn truncate(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else {
        format!("{}...", &s[..max_width - 3])
    }
}

/// Indent text by a specified number of spaces
pub fn indent(text: &str, spaces: usize) -> String {
    let indent = " ".repeat(spaces);
    text.lines()
        .map(|line| format!("{}{}", indent, line))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_format_config_default() {
        let config = FormatConfig::default();
        assert_eq!(config.output_format, OutputFormat::Plain);
        assert!(config.show_timestamps);
        assert!(!config.show_metadata);
        assert_eq!(config.indent_size, 2);
    }

    #[test]
    fn test_format_timestamp() {
        let dt = Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap();
        let formatted = format_timestamp(&dt);
        assert!(formatted.contains("2024-01-15"));
        assert!(formatted.contains("10:30:00"));
    }

    #[test]
    fn test_format_relative_time_just_now() {
        let now = Utc::now();
        let formatted = format_relative_time(&now);
        assert_eq!(formatted, "just now");
    }

    #[test]
    fn test_format_relative_time_hours_ago() {
        let past = Utc::now() - Duration::hours(3);
        let formatted = format_relative_time(&past);
        assert_eq!(formatted, "3 hours ago");
    }

    #[test]
    fn test_format_relative_time_days_ago() {
        let past = Utc::now() - Duration::days(5);
        let formatted = format_relative_time(&past);
        assert_eq!(formatted, "5 days ago");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(Duration::seconds(30)), "30s");
        assert_eq!(format_duration(Duration::seconds(90)), "1m 30s");
        assert_eq!(format_duration(Duration::seconds(3600)), "1h");
        assert_eq!(format_duration(Duration::seconds(3660)), "1h 1m");
    }

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(500), "500 B");
        assert_eq!(format_size(1024), "1.00 KB");
        assert_eq!(format_size(1048576), "1.00 MB");
        assert_eq!(format_size(1073741824), "1.00 GB");
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(42), "42");
    }

    #[test]
    fn test_pad() {
        assert_eq!(pad("hello", 10), "hello     ");
        assert_eq!(pad("hello", 3), "hello");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_indent() {
        let text = "line1\nline2\nline3";
        let indented = indent(text, 2);
        assert!(indented.contains("  line1"));
        assert!(indented.contains("  line2"));
    }
}
