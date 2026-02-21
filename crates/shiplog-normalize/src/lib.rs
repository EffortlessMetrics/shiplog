//! Normalization utilities for shiplog.
//!
//! This crate provides normalization utilities for the shiplog ecosystem.

/// Normalizes a string by converting it to lowercase and trimming whitespace.
///
/// # Arguments
/// * `input` - The string to normalize
///
/// # Returns
/// * `String` - The normalized string
pub fn normalize_string(input: &str) -> String {
    input.trim().to_lowercase()
}

/// Normalizes a path by collapsing multiple slashes and removing trailing slashes.
///
/// # Arguments
/// * `input` - The path to normalize
///
/// # Returns
/// * `String` - The normalized path
pub fn normalize_path(input: &str) -> String {
    let mut result = input.trim().to_string();

    // Replace multiple slashes with single slash
    while result.contains("//") {
        result = result.replace("//", "/");
    }

    // Remove trailing slash (but keep root)
    if result.len() > 1 && result.ends_with('/') {
        result.pop();
    }

    result
}

/// Normalizes whitespace by replacing multiple spaces with a single space.
///
/// # Arguments
/// * `input` - The string to normalize
///
/// # Returns
/// * `String` - The normalized string
pub fn normalize_whitespace(input: &str) -> String {
    let mut result = String::new();
    let mut last_was_space = false;

    for c in input.chars() {
        if c.is_whitespace() {
            if !last_was_space && !result.is_empty() {
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

/// Normalizes a line ending to Unix-style (LF).
///
/// # Arguments
/// * `input` - The string to normalize
///
/// # Returns
/// * `String` - The normalized string
pub fn normalize_line_endings(input: &str) -> String {
    input.replace("\r\n", "\n").replace("\r", "\n")
}

/// Normalizes a boolean string to a canonical form.
///
/// # Arguments
/// * `input` - The boolean string to normalize
///
/// # Returns
/// * `String` - "true" or "false"
pub fn normalize_bool(input: &str) -> String {
    let normalized = input.trim().to_lowercase();
    match normalized.as_str() {
        "true" | "1" | "yes" | "on" | "enabled" => "true".to_string(),
        _ => "false".to_string(),
    }
}

/// Normalizes a number by removing unnecessary decimal points and leading zeros.
///
/// # Arguments
/// * `input` - The number string to normalize
///
/// # Returns
/// * `String` - The normalized number string
pub fn normalize_number(input: &str) -> String {
    let trimmed = input.trim();

    // Handle negative numbers
    if let Some(stripped) = trimmed.strip_prefix('-') {
        let abs_normalized = normalize_number(stripped);
        if abs_normalized.is_empty() {
            return "0".to_string();
        }
        return format!("-{}", abs_normalized);
    }

    // Remove leading zeros
    let result = trimmed.to_string();

    // Handle decimal numbers
    if let Some(dot_pos) = result.find('.') {
        let int_part = &result[..dot_pos];
        let frac_part = &result[dot_pos..];

        let normalized_int = int_part.trim_start_matches('0');
        let int_result = if normalized_int.is_empty() {
            "0"
        } else {
            normalized_int
        };

        // Remove trailing zeros from fractional part
        let frac = frac_part.trim_end_matches('0');
        let frac_result = if frac == "." { "" } else { frac };

        return format!("{}{}", int_result, frac_result);
    }

    // Integer case
    let normalized = result.trim_start_matches('0');
    if normalized.is_empty() {
        "0".to_string()
    } else {
        normalized.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_string() {
        assert_eq!(normalize_string("  HELLO  "), "hello");
        assert_eq!(normalize_string("World"), "world");
    }

    #[test]
    fn test_normalize_path() {
        assert_eq!(normalize_path("foo/bar//baz/"), "foo/bar/baz");
        assert_eq!(normalize_path("/foo/bar/"), "/foo/bar");
        assert_eq!(normalize_path("foo"), "foo");
    }

    #[test]
    fn test_normalize_whitespace() {
        assert_eq!(normalize_whitespace("hello    world"), "hello world");
        assert_eq!(normalize_whitespace("  a   b   c  "), "a b c");
    }

    #[test]
    fn test_normalize_line_endings() {
        assert_eq!(normalize_line_endings("hello\r\nworld"), "hello\nworld");
        assert_eq!(normalize_line_endings("hello\rworld"), "hello\nworld");
        assert_eq!(normalize_line_endings("hello\nworld"), "hello\nworld");
    }

    #[test]
    fn test_normalize_bool() {
        assert_eq!(normalize_bool("true"), "true");
        assert_eq!(normalize_bool("1"), "true");
        assert_eq!(normalize_bool("yes"), "true");
        assert_eq!(normalize_bool("false"), "false");
        assert_eq!(normalize_bool("0"), "false");
        assert_eq!(normalize_bool("no"), "false");
    }

    #[test]
    fn test_normalize_number() {
        assert_eq!(normalize_number("00123"), "123");
        assert_eq!(normalize_number("001.2300"), "1.23");
        assert_eq!(normalize_number("-007"), "-7");
        assert_eq!(normalize_number("0"), "0");
    }
}
