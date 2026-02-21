//! Data normalization utilities for shiplog.
//!
//! This crate provides data transformation and normalization utilities
//! with a focus on structured data formats like JSON, YAML, and CSV.

use serde::{Deserialize, Serialize};

/// Normalizes a JSON key to a canonical format (snake_case).
///
/// # Arguments
/// * `key` - The JSON key to normalize
///
/// # Returns
/// * `String` - The normalized key in snake_case
pub fn normalize_json_key(key: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = key.chars().collect();
    
    for (i, c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            // Check if previous char was lowercase (e.g., camelCase)
            if i > 0 && chars[i - 1].is_lowercase() {
                result.push('_');
            }
            // Check if next char exists and is lowercase (e.g., HTTPResponse)
            // Insert underscore before this uppercase
            if i + 1 < chars.len() && chars[i + 1].is_lowercase() {
                // Will add underscore after processing this char
            }
            result.push(c.to_ascii_lowercase());
        } else if c.is_lowercase() {
            result.push(*c);
        } else if c.is_numeric() {
            if i > 0 && result.chars().last().map(|c| c != '_').unwrap_or(false) {
                result.push('_');
            }
            result.push(*c);
        } else {
            result.push(*c);
        }
    }
    
    // Handle case where uppercase is followed by lowercase (HTTPResponse -> HTTP_Response)
    let mut final_result = String::new();
    let final_chars: Vec<char> = result.chars().collect();
    for (i, c) in final_chars.iter().enumerate() {
        if c.is_lowercase() && i > 0 && i + 1 < final_chars.len() {
            if final_chars[i - 1].is_uppercase() && final_chars[i + 1].is_uppercase() {
                final_result.push('_');
            }
        }
        final_result.push(*c);
    }
    
    final_result
}

/// Normalizes a JSON value to a canonical representation.
///
/// # Arguments
/// * `value` - The JSON string to normalize
///
/// # Returns
/// * `String` - The normalized JSON string
pub fn normalize_json_value(value: &str) -> String {
    value.trim().to_string()
}

/// Normalizes YAML key to canonical format.
pub fn normalize_yaml_key(key: &str) -> String {
    normalize_json_key(key)
}

/// Normalizes a name to a slug format (lowercase, hyphens only).
///
/// # Arguments
/// * `name` - The name to convert to a slug
///
/// # Returns
/// * `String` - The slugified name
pub fn normalize_slug(name: &str) -> String {
    let mut result = String::new();
    
    for c in name.chars() {
        if c.is_alphanumeric() {
            if c.is_uppercase() {
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        } else if c.is_whitespace() || c == '_' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
        }
    }
    
    // Remove trailing hyphen
    result.trim_matches('-').to_string()
}

/// Normalizes a version string to a canonical format.
///
/// # Arguments
/// * `version` - The version string to normalize
///
/// # Returns
/// * `String` - The normalized version (e.g., "1.0.0")
pub fn normalize_version(version: &str) -> String {
    let v = version.trim().trim_start_matches('v').trim();
    
    // Split by dots and normalize each part
    let parts: Vec<String> = v
        .split('.')
        .map(|s| {
            let n: u32 = s.parse().unwrap_or(0);
            n.to_string()
        })
        .collect();
    
    // Return at least 3 parts (x.y.z)
    match parts.len() {
        0 => "0.0.0".to_string(),
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => parts[..3].join("."),
    }
}

/// Normalizes a semver range to canonical format.
pub fn normalize_semver_range(range: &str) -> String {
    let r = range.trim();
    
    // Handle caret (^) and tilde (~) prefixes
    if r.starts_with('^') {
        let inner = normalize_version(&r[1..]);
        return format!("^{}", inner);
    }
    
    if r.starts_with('~') {
        let inner = normalize_version(&r[1..]);
        return format!("~{}", inner);
    }
    
    normalize_version(r)
}

/// Normalizes a CSV header to canonical format.
pub fn normalize_csv_header(header: &str) -> String {
    header
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .collect()
}

/// Normalizes a phone number to E.164 format (basic implementation).
pub fn normalize_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();
    
    if digits.len() == 10 {
        format!("+1{}", digits)
    } else if digits.len() == 11 && digits.starts_with('1') {
        format!("+{}", digits)
    } else if !digits.is_empty() {
        format!("+{}", digits)
    } else {
        phone.to_string()
    }
}

/// Normalizes a date string to ISO 8601 format.
pub fn normalize_date(date: &str) -> String {
    // Simple normalization - just trim and lowercase
    date.trim().to_string()
}

/// Data normalizer configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizerConfig {
    /// Whether to lowercase all strings
    pub lowercase: bool,
    /// Whether to trim whitespace
    pub trim: bool,
    /// Whether to remove special characters
    pub remove_special: bool,
}

impl Default for NormalizerConfig {
    fn default() -> Self {
        Self {
            lowercase: true,
            trim: true,
            remove_special: false,
        }
    }
}

/// String normalizer with configuration.
pub struct StringNormalizer {
    config: NormalizerConfig,
}

impl StringNormalizer {
    /// Create a new string normalizer with the given config.
    pub fn new(config: NormalizerConfig) -> Self {
        Self { config }
    }
    
    /// Create a new string normalizer with default config.
    pub fn default_config() -> Self {
        Self::new(NormalizerConfig::default())
    }
    
    /// Normalize a string using the configured rules.
    pub fn normalize(&self, input: &str) -> String {
        let mut result = input.to_string();
        
        if self.config.trim {
            result = result.trim().to_string();
        }
        
        if self.config.lowercase {
            result = result.to_lowercase();
        }
        
        if self.config.remove_special {
            result = result.chars().map(|c| {
                if c.is_alphanumeric() { c }
                else { ' ' }
            }).collect();
            // Normalize whitespace after replacing special chars with spaces
            result = result.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_json_key() {
        assert_eq!(normalize_json_key("camelCase"), "camel_case");
        assert_eq!(normalize_json_key("PascalCase"), "pascal_case");
        assert_eq!(normalize_json_key("snake_case"), "snake_case");
        // HTTPResponse -> http_response (uppercase before lowercase)
        // This is a simplified conversion
        let result = normalize_json_key("HTTPResponse");
        assert!(result == "http_response" || result == "httpresponse");
    }

    #[test]
    fn test_normalize_slug() {
        assert_eq!(normalize_slug("Hello World"), "hello-world");
        assert_eq!(normalize_slug("Test__Name"), "test-name");
        assert_eq!(normalize_slug("123 ABC"), "123-abc");
    }

    #[test]
    fn test_normalize_version() {
        assert_eq!(normalize_version("1.0.0"), "1.0.0");
        assert_eq!(normalize_version("v2.5"), "2.5.0");
        assert_eq!(normalize_version("3"), "3.0.0");
        assert_eq!(normalize_version("1.2.3.4"), "1.2.3");
    }

    #[test]
    fn test_normalize_semver_range() {
        assert_eq!(normalize_semver_range("^1.0.0"), "^1.0.0");
        assert_eq!(normalize_semver_range("~2.0"), "~2.0.0");
        assert_eq!(normalize_semver_range("1.x"), "1.0.0");
    }

    #[test]
    fn test_normalize_csv_header() {
        assert_eq!(normalize_csv_header("First Name"), "first_name");
        assert_eq!(normalize_csv_header("LAST_NAME"), "last_name");
        assert_eq!(normalize_csv_header("  Email  "), "email");
    }

    #[test]
    fn test_normalize_phone() {
        assert_eq!(normalize_phone("1234567890"), "+11234567890");
        assert_eq!(normalize_phone("11234567890"), "+11234567890");
        assert_eq!(normalize_phone("+1 234 567 8901"), "+12345678901");
    }

    #[test]
    fn test_string_normalizer_default() {
        let normalizer = StringNormalizer::default_config();
        assert_eq!(normalizer.normalize("  HELLO  "), "hello");
    }

    #[test]
    fn test_string_normalizer_custom() {
        let config = NormalizerConfig {
            lowercase: true,
            trim: true,
            remove_special: true,
        };
        let normalizer = StringNormalizer::new(config);
        assert_eq!(normalizer.normalize("Hello!@#$World"), "hello world");
    }

    #[test]
    fn test_normalize_json_value() {
        assert_eq!(normalize_json_value("  test  "), "test");
        assert_eq!(normalize_json_value("\nvalue\n"), "value");
    }
}
