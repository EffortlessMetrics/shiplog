//! Integration tests for shiplog-sanitize.

use proptest::prelude::*;
use shiplog_sanitize::*;

// ── remove_control_characters tests ─────────────────────────────

#[test]
fn remove_control_preserves_whitespace() {
    assert_eq!(
        remove_control_characters("hello\tworld\n"),
        "hello\tworld\n"
    );
}

#[test]
fn remove_control_strips_null() {
    assert_eq!(remove_control_characters("hel\x00lo"), "hello");
}

#[test]
fn remove_control_strips_bell() {
    assert_eq!(remove_control_characters("hel\x07lo"), "hello");
}

#[test]
fn remove_control_strips_escape() {
    assert_eq!(remove_control_characters("hel\x1blo"), "hello");
}

#[test]
fn remove_control_empty() {
    assert_eq!(remove_control_characters(""), "");
}

#[test]
fn remove_control_only_controls() {
    assert_eq!(remove_control_characters("\x00\x01\x02"), "");
}

#[test]
fn remove_control_preserves_unicode() {
    assert_eq!(remove_control_characters("日本語\x00"), "日本語");
}

// ── remove_non_ascii tests ──────────────────────────────────────

#[test]
fn remove_non_ascii_basic() {
    assert_eq!(remove_non_ascii("hello world"), "hello world");
}

#[test]
fn remove_non_ascii_strips_unicode() {
    assert_eq!(remove_non_ascii("hello 世界"), "hello ");
}

#[test]
fn remove_non_ascii_strips_emoji() {
    assert_eq!(remove_non_ascii("hello 🌍"), "hello ");
}

#[test]
fn remove_non_ascii_preserves_special_ascii() {
    assert_eq!(remove_non_ascii("a!@#$%^&*()"), "a!@#$%^&*()");
}

#[test]
fn remove_non_ascii_empty() {
    assert_eq!(remove_non_ascii(""), "");
}

#[test]
fn remove_non_ascii_all_unicode() {
    assert_eq!(remove_non_ascii("日本語"), "");
}

// ── remove_non_alphanumeric tests ───────────────────────────────

#[test]
fn remove_non_alphanumeric_basic() {
    assert_eq!(remove_non_alphanumeric("hello world!"), "helloworld");
}

#[test]
fn remove_non_alphanumeric_preserves_digits() {
    assert_eq!(remove_non_alphanumeric("a1b2c3"), "a1b2c3");
}

#[test]
fn remove_non_alphanumeric_unicode() {
    // Unicode letters are alphanumeric
    assert_eq!(remove_non_alphanumeric("hello 世界!"), "hello世界");
}

#[test]
fn remove_non_alphanumeric_empty() {
    assert_eq!(remove_non_alphanumeric(""), "");
}

#[test]
fn remove_non_alphanumeric_only_special() {
    assert_eq!(remove_non_alphanumeric("!@#$%^&*()"), "");
}

// ── sanitize_filename tests ─────────────────────────────────────

#[test]
fn sanitize_filename_safe_chars() {
    assert_eq!(sanitize_filename("my-file_123.txt"), "my-file_123.txt");
}

#[test]
fn sanitize_filename_replaces_unsafe() {
    assert_eq!(sanitize_filename("file:name.txt"), "file_name.txt");
    assert_eq!(sanitize_filename("file/name.txt"), "file_name.txt");
    assert_eq!(sanitize_filename("file\\name.txt"), "file_name.txt");
    assert_eq!(sanitize_filename("file<name>.txt"), "file_name_.txt");
}

#[test]
fn sanitize_filename_spaces() {
    assert_eq!(sanitize_filename("my file.txt"), "my_file.txt");
}

#[test]
fn sanitize_filename_empty() {
    assert_eq!(sanitize_filename(""), "");
}

#[test]
fn sanitize_filename_unicode() {
    let result = sanitize_filename("日本語.txt");
    // Alphanumeric unicode should be preserved, others replaced
    assert!(result.ends_with(".txt"));
}

#[test]
fn sanitize_filename_all_unsafe() {
    assert_eq!(sanitize_filename(":<>|?*"), "______");
}

#[test]
fn sanitize_filename_dots_preserved() {
    assert_eq!(sanitize_filename("file.tar.gz"), "file.tar.gz");
}

#[test]
fn sanitize_filename_hyphen_underscore_preserved() {
    assert_eq!(sanitize_filename("a-b_c"), "a-b_c");
}

// ── escape_shell tests ──────────────────────────────────────────

#[test]
fn escape_shell_simple() {
    assert_eq!(escape_shell("hello"), "'hello'");
}

#[test]
fn escape_shell_with_spaces() {
    assert_eq!(escape_shell("hello world"), "'hello world'");
}

#[test]
fn escape_shell_with_single_quote() {
    assert_eq!(escape_shell("it's"), "'it'\\''s'");
}

#[test]
fn escape_shell_empty() {
    assert_eq!(escape_shell(""), "''");
}

#[test]
fn escape_shell_special_chars() {
    let result = escape_shell("$(rm -rf /)");
    assert!(result.starts_with('\''));
    assert!(result.ends_with('\''));
}

#[test]
fn escape_shell_multiple_single_quotes() {
    // Input: '' -> wraps in quotes, each ' becomes '\''
    assert_eq!(escape_shell("''"), "''\\'''\\'''");
}

// ── sanitize_whitespace tests ───────────────────────────────────

#[test]
fn sanitize_whitespace_collapses() {
    assert_eq!(sanitize_whitespace("hello   world"), "hello world");
}

#[test]
fn sanitize_whitespace_trims() {
    assert_eq!(sanitize_whitespace("  hello  "), "hello");
}

#[test]
fn sanitize_whitespace_tabs_and_newlines() {
    assert_eq!(sanitize_whitespace("a\t\nb"), "a b");
}

#[test]
fn sanitize_whitespace_empty() {
    assert_eq!(sanitize_whitespace(""), "");
}

#[test]
fn sanitize_whitespace_only_whitespace() {
    assert_eq!(sanitize_whitespace("   \t\n  "), "");
}

#[test]
fn sanitize_whitespace_single_word() {
    assert_eq!(sanitize_whitespace("hello"), "hello");
}

// ── remove_null_bytes tests ─────────────────────────────────────

#[test]
fn remove_null_bytes_basic() {
    assert_eq!(remove_null_bytes("hello\0world"), "helloworld");
}

#[test]
fn remove_null_bytes_no_nulls() {
    assert_eq!(remove_null_bytes("hello world"), "hello world");
}

#[test]
fn remove_null_bytes_only_nulls() {
    assert_eq!(remove_null_bytes("\0\0\0"), "");
}

#[test]
fn remove_null_bytes_empty() {
    assert_eq!(remove_null_bytes(""), "");
}

#[test]
fn remove_null_bytes_preserves_unicode() {
    assert_eq!(remove_null_bytes("日\0本\0語"), "日本語");
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_sanitize_filename_matrix() {
    let inputs = [
        "normal.txt",
        "my file.txt",
        "file:name.txt",
        "path/to/file.txt",
        "a<b>c|d?e*f",
        "日本語.txt",
        "",
    ];
    let formatted: Vec<String> = inputs
        .iter()
        .map(|&s| format!("{:>25} => {}", format!("\"{}\"", s), sanitize_filename(s)))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

#[test]
fn snapshot_escape_shell_matrix() {
    let inputs = [
        "hello",
        "hello world",
        "it's a test",
        "$(danger)",
        "",
        "normal",
    ];
    let formatted: Vec<String> = inputs
        .iter()
        .map(|&s| format!("{:>20} => {}", format!("\"{}\"", s), escape_shell(s)))
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_remove_control_idempotent(s in "\\PC{0,100}") {
        let once = remove_control_characters(&s);
        let twice = remove_control_characters(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_remove_non_ascii_idempotent(s in "\\PC{0,100}") {
        let once = remove_non_ascii(&s);
        let twice = remove_non_ascii(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_remove_non_ascii_result_is_ascii(s in "\\PC{0,100}") {
        let result = remove_non_ascii(&s);
        prop_assert!(result.is_ascii());
    }

    #[test]
    fn prop_remove_non_alphanumeric_idempotent(s in "\\PC{0,100}") {
        let once = remove_non_alphanumeric(&s);
        let twice = remove_non_alphanumeric(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_remove_non_alphanumeric_result_is_alphanumeric(s in "\\PC{0,100}") {
        let result = remove_non_alphanumeric(&s);
        prop_assert!(result.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn prop_sanitize_filename_idempotent(s in "[a-zA-Z0-9._-]{0,50}") {
        let once = sanitize_filename(&s);
        let twice = sanitize_filename(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_sanitize_filename_safe_chars_only(s in "\\PC{0,50}") {
        let result = sanitize_filename(&s);
        for c in result.chars() {
            prop_assert!(
                c.is_alphanumeric() || c == '-' || c == '_' || c == '.',
                "Unexpected character: {:?}",
                c
            );
        }
    }

    #[test]
    fn prop_sanitize_filename_preserves_length(s in "\\PC{0,50}") {
        let result = sanitize_filename(&s);
        prop_assert_eq!(result.chars().count(), s.chars().count());
    }

    #[test]
    fn prop_sanitize_whitespace_idempotent(s in "\\PC{0,50}") {
        let once = sanitize_whitespace(&s);
        let twice = sanitize_whitespace(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_sanitize_whitespace_no_leading_trailing(s in "\\PC{0,50}") {
        let result = sanitize_whitespace(&s);
        if !result.is_empty() {
            prop_assert!(!result.starts_with(' '));
            prop_assert!(!result.ends_with(' '));
        }
    }

    #[test]
    fn prop_remove_null_bytes_idempotent(s in "\\PC{0,100}") {
        let once = remove_null_bytes(&s);
        let twice = remove_null_bytes(&once);
        prop_assert_eq!(once, twice);
    }

    #[test]
    fn prop_remove_null_bytes_no_nulls(s in "\\PC{0,100}") {
        let result = remove_null_bytes(&s);
        prop_assert!(!result.contains('\0'));
    }

    #[test]
    fn prop_escape_shell_wraps_in_quotes(s in "[a-zA-Z0-9 ]{0,30}") {
        let result = escape_shell(&s);
        prop_assert!(result.starts_with('\''));
        prop_assert!(result.ends_with('\''));
    }
}
