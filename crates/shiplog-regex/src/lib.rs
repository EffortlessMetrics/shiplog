//! Regex utilities for shiplog.
//!
//! This crate provides regex utilities for the shiplog ecosystem.

use anyhow::Result;
use regex::Regex;

/// Compiles a regex pattern
pub fn compile(pattern: &str) -> Result<Regex> {
    Ok(Regex::new(pattern)?)
}

/// Checks if a string matches a pattern
pub fn is_match(pattern: &str, text: &str) -> Result<bool> {
    let re = compile(pattern)?;
    Ok(re.is_match(text))
}

/// Finds all matches of a pattern in a text
pub fn find_all(pattern: &str, text: &str) -> Result<Vec<String>> {
    let re = compile(pattern)?;
    Ok(re.find_iter(text).map(|m| m.as_str().to_string()).collect())
}

/// Replaces all matches of a pattern with a replacement string
pub fn replace_all(pattern: &str, text: &str, replacement: &str) -> Result<String> {
    let re = compile(pattern)?;
    Ok(re.replace_all(text, replacement).to_string())
}

/// Captures groups from a match
pub fn capture_groups(pattern: &str, text: &str) -> Result<Vec<Vec<String>>> {
    let re = compile(pattern)?;
    let mut results = Vec::new();

    for cap in re.captures_iter(text) {
        let groups: Vec<String> = cap
            .iter()
            .skip(1) // Skip the full match
            .filter_map(|m| m.map(|m| m.as_str().to_string()))
            .collect();
        results.push(groups);
    }

    Ok(results)
}

/// Splits text by a pattern
pub fn split(pattern: &str, text: &str) -> Result<Vec<String>> {
    let re = compile(pattern)?;
    Ok(re.split(text).map(|s| s.to_string()).collect())
}

/// Checks if a pattern is valid regex
pub fn is_valid_pattern(pattern: &str) -> bool {
    Regex::new(pattern).is_ok()
}

/// Counts the number of matches in a text
pub fn count_matches(pattern: &str, text: &str) -> Result<usize> {
    let re = compile(pattern)?;
    Ok(re.find_iter(text).count())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile() {
        let re = compile(r"\d+").unwrap();
        assert!(re.is_match("123"));
    }

    #[test]
    fn test_is_match() {
        assert!(is_match(r"\d+", "123").unwrap());
        assert!(!is_match(r"\d+", "abc").unwrap());
    }

    #[test]
    fn test_find_all() {
        let matches = find_all(r"\d+", "123 abc 456 def 789").unwrap();
        assert_eq!(matches, vec!["123", "456", "789"]);
    }

    #[test]
    fn test_replace_all() {
        let result = replace_all(r"\d+", "abc123def456", "X").unwrap();
        assert_eq!(result, "abcXdefX");
    }

    #[test]
    fn test_capture_groups() {
        let groups = capture_groups(r"(\d+)-(\d+)", "123-456 789-012").unwrap();
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0], vec!["123", "456"]);
        assert_eq!(groups[1], vec!["789", "012"]);
    }

    #[test]
    fn test_split() {
        let parts = split(r"[,;]", "a,b;c,d").unwrap();
        assert_eq!(parts, vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn test_is_valid_pattern() {
        assert!(is_valid_pattern(r"\d+"));
        assert!(is_valid_pattern(r"[a-z]+"));
        assert!(!is_valid_pattern(r"["));
    }

    #[test]
    fn test_count_matches() {
        assert_eq!(count_matches(r"\d+", "123 abc 456").unwrap(), 2);
        assert_eq!(count_matches(r"\d+", "abc def").unwrap(), 0);
    }
}
