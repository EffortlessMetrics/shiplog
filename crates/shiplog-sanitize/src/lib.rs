//! Sanitization utilities for shiplog.
//!
//! This crate provides sanitization utilities for the shiplog ecosystem.

/// Removes control characters from a string (except whitespace).
pub fn remove_control_characters(input: &str) -> String {
    input.chars().filter(|c| !c.is_control() || c.is_whitespace()).collect()
}

/// Removes non-ASCII characters from a string.
pub fn remove_non_ascii(input: &str) -> String {
    input.chars().filter(|c| c.is_ascii()).collect()
}

/// Removes non-alphanumeric characters from a string.
pub fn remove_non_alphanumeric(input: &str) -> String {
    input.chars().filter(|c| c.is_alphanumeric()).collect()
}

/// Removes special characters that could be dangerous in filenames.
pub fn sanitize_filename(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

/// Escapes special characters for shell commands.
pub fn escape_shell(input: &str) -> String {
    let mut result = String::new();
    result.push('\'');
    for c in input.chars() {
        if c == '\'' {
            result.push_str("'\\''");
        } else {
            result.push(c);
        }
    }
    result.push('\'');
    result
}

/// Removes leading/trailing whitespace and collapses internal whitespace.
pub fn sanitize_whitespace(input: &str) -> String {
    let mut result = String::new();
    let mut last_was_space = false;
    
    for c in input.chars() {
        if c.is_whitespace() {
            if !last_was_space {
                result.push(' ');
                last_was_space = true;
            }
        } else {
            result.push(c);
            last_was_space = false;
        }
    }
    
    result.trim().to_string()
}

/// Replaces null bytes with an empty string.
pub fn remove_null_bytes(input: &str) -> String {
    input.replace('\0', "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_control_characters() {
        assert_eq!(remove_control_characters("hello\tworld\n"), "hello\tworld\n");
        assert_eq!(remove_control_characters("hel\x00lo"), "hello");
    }

    #[test]
    fn test_remove_non_ascii() {
        assert_eq!(remove_non_ascii("hello world"), "hello world");
        assert_eq!(remove_non_ascii("hello"), "hello");
    }

    #[test]
    fn test_remove_non_alphanumeric() {
        assert_eq!(remove_non_alphanumeric("hello world!"), "helloworld");
        assert_eq!(remove_non_alphanumeric("a1b2c3"), "a1b2c3");
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("file:name.txt"), "file_name.txt");
        assert_eq!(sanitize_filename("my-file_123.txt"), "my-file_123.txt");
    }

    #[test]
    fn test_escape_shell() {
        assert_eq!(escape_shell("hello"), "'hello'");
        assert_eq!(escape_shell("hello world"), "'hello world'");
        assert_eq!(escape_shell("it's"), "'it'\\''s'");
    }

    #[test]
    fn test_sanitize_whitespace() {
        assert_eq!(sanitize_whitespace("  hello   world  "), "hello world");
        assert_eq!(sanitize_whitespace("a\t\nb"), "a b");
    }

    #[test]
    fn test_remove_null_bytes() {
        assert_eq!(remove_null_bytes("hello\0world"), "helloworld");
        assert_eq!(remove_null_bytes("no nulls"), "no nulls");
    }
}
