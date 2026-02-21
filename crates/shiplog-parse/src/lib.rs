//! Parsing utilities for shiplog.
//!
//! This crate provides parsing utilities for the shiplog ecosystem.

/// Parses a string into a u64 integer.
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Ok(u64)` - The parsed integer
/// * `Err(String)` - If parsing fails
pub fn parse_u64(input: &str) -> Result<u64, String> {
    input
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse u64: {}", e))
}

/// Parses a string into an i64 integer.
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Ok(i64)` - The parsed integer
/// * `Err(String)` - If parsing fails
pub fn parse_i64(input: &str) -> Result<i64, String> {
    input
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse i64: {}", e))
}

/// Parses a string into a f64 floating-point number.
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Ok(f64)` - The parsed number
/// * `Err(String)` - If parsing fails
pub fn parse_f64(input: &str) -> Result<f64, String> {
    input
        .trim()
        .parse()
        .map_err(|e| format!("Failed to parse f64: {}", e))
}

/// Parses a boolean string (true/false, 0/1, yes/no).
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Ok(bool)` - The parsed boolean
/// * `Err(String)` - If parsing fails
pub fn parse_bool(input: &str) -> Result<bool, String> {
    let input = input.trim().to_lowercase();
    match input.as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => Err(format!("Failed to parse boolean: {}", input)),
    }
}

/// Parses a comma-separated list of strings.
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Vec<String>` - The list of parsed strings
pub fn parse_comma_separated(input: &str) -> Vec<String> {
    input
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parses a key-value pair in the format "key=value".
///
/// # Arguments
/// * `input` - The string to parse
///
/// # Returns
/// * `Ok((String, String))` - The parsed key and value
/// * `Err(String)` - If parsing fails
pub fn parse_key_value(input: &str) -> Result<(String, String), String> {
    let parts: Vec<&str> = input.splitn(2, '=').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid key-value format: {}", input));
    }
    Ok((parts[0].trim().to_string(), parts[1].trim().to_string()))
}

/// Trims whitespace from both ends of a string.
///
/// # Arguments
/// * `input` - The string to trim
///
/// # Returns
/// * `String` - The trimmed string
pub fn trim(input: &str) -> String {
    input.trim().to_string()
}

/// Removes leading and trailing quotes from a string.
///
/// # Arguments
/// * `input` - The string to unquote
///
/// # Returns
/// * `String` - The unquoted string
pub fn unquote(input: &str) -> String {
    let trimmed = input.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64("123").unwrap(), 123);
        assert_eq!(parse_u64("  456  ").unwrap(), 456);
        assert!(parse_u64("abc").is_err());
    }

    #[test]
    fn test_parse_i64() {
        assert_eq!(parse_i64("-123").unwrap(), -123);
        assert_eq!(parse_i64("456").unwrap(), 456);
        assert!(parse_i64("abc").is_err());
    }

    #[test]
    fn test_parse_f64() {
        assert!((parse_f64("3.14").unwrap() - std::f64::consts::PI).abs() < 0.01);
        assert!((parse_f64("-2.5").unwrap() - (-2.5)).abs() < 0.001);
        assert!(parse_f64("abc").is_err());
    }

    #[test]
    fn test_parse_bool() {
        assert!(parse_bool("true").unwrap());
        assert!(parse_bool("1").unwrap());
        assert!(parse_bool("yes").unwrap());
        assert!(!parse_bool("false").unwrap());
        assert!(!parse_bool("0").unwrap());
        assert!(!parse_bool("no").unwrap());
        assert!(parse_bool("invalid").is_err());
    }

    #[test]
    fn test_parse_comma_separated() {
        assert_eq!(parse_comma_separated("a,b,c"), vec!["a", "b", "c"]);
        assert_eq!(parse_comma_separated(" a , b , c "), vec!["a", "b", "c"]);
        assert_eq!(parse_comma_separated(""), Vec::<String>::new());
    }

    #[test]
    fn test_parse_key_value() {
        assert_eq!(
            parse_key_value("key=value").unwrap(),
            ("key".to_string(), "value".to_string())
        );
        assert_eq!(
            parse_key_value("foo=bar=baz").unwrap(),
            ("foo".to_string(), "bar=baz".to_string())
        );
        assert!(parse_key_value("invalid").is_err());
    }

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("world"), "world");
    }

    #[test]
    fn test_unquote() {
        assert_eq!(unquote("\"hello\""), "hello");
        assert_eq!(unquote("'world'"), "world");
        assert_eq!(unquote("noquotes"), "noquotes");
    }
}
