//! String utilities for shiplog.
//!
//! This crate provides string manipulation utilities for the shiplog ecosystem.

/// Trims whitespace from both ends of a string
pub fn trim(s: &str) -> String {
    s.trim().to_string()
}

/// Converts a string to title case
pub fn to_title_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut capitalize_next = true;
    
    for c in s.chars() {
        if c.is_whitespace() || c == '_' || c == '-' {
            capitalize_next = true;
            result.push(c);
        } else if capitalize_next {
            result.extend(c.to_uppercase());
            capitalize_next = false;
        } else {
            result.extend(c.to_lowercase());
        }
    }
    result
}

/// Converts a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('_');
        }
        if c.is_alphanumeric() {
            result.extend(c.to_lowercase());
        } else if c.is_whitespace() || c == '-' {
            result.push('_');
        }
    }
    
    // Remove consecutive underscores
    result.split('_').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("_")
}

/// Converts a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        if c.is_alphanumeric() {
            result.extend(c.to_lowercase());
        } else if c.is_whitespace() || c == '_' {
            result.push('-');
        }
    }
    
    // Remove consecutive dashes
    result.split('-').filter(|s| !s.is_empty()).collect::<Vec<_>>().join("-")
}

/// Checks if a string is empty or contains only whitespace
pub fn is_blank(s: &str) -> bool {
    s.trim().is_empty()
}

/// Reverses a string
pub fn reverse(s: &str) -> String {
    s.chars().rev().collect()
}

/// Counts the number of words in a string
pub fn word_count(s: &str) -> usize {
    s.split_whitespace().count()
}

/// Removes all whitespace from a string
pub fn remove_whitespace(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// Left-pads a string to a minimum length
pub fn pad_left(s: &str, min_len: usize, pad_char: char) -> String {
    if s.len() >= min_len {
        s.to_string()
    } else {
        pad_char.to_string().repeat(min_len - s.len()) + s
    }
}

/// Right-pads a string to a minimum length
pub fn pad_right(s: &str, min_len: usize, pad_char: char) -> String {
    if s.len() >= min_len {
        s.to_string()
    } else {
        s.to_string() + &pad_char.to_string().repeat(min_len - s.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trim() {
        assert_eq!(trim("  hello  "), "hello");
        assert_eq!(trim("hello"), "hello");
    }

    #[test]
    fn test_to_title_case() {
        assert_eq!(to_title_case("hello world"), "Hello World");
        assert_eq!(to_title_case("the quick brown fox"), "The Quick Brown Fox");
    }

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("HelloWorld"), "hello_world");
        assert_eq!(to_snake_case("Hello World"), "hello_world");
        assert_eq!(to_snake_case("hello-world"), "hello_world");
    }

    #[test]
    fn test_to_kebab_case() {
        assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
        assert_eq!(to_kebab_case("hello_world"), "hello-world");
        assert_eq!(to_kebab_case("Hello World"), "hello-world");
    }

    #[test]
    fn test_is_blank() {
        assert!(is_blank("   "));
        assert!(is_blank(""));
        assert!(!is_blank("hello"));
    }

    #[test]
    fn test_reverse() {
        assert_eq!(reverse("hello"), "olleh");
        assert_eq!(reverse("a"), "a");
    }

    #[test]
    fn test_word_count() {
        assert_eq!(word_count("hello world"), 2);
        assert_eq!(word_count("  one   two  three  "), 3);
    }

    #[test]
    fn test_remove_whitespace() {
        assert_eq!(remove_whitespace("hello world"), "helloworld");
        assert_eq!(remove_whitespace("a b c d"), "abcd");
    }

    #[test]
    fn test_pad_left() {
        assert_eq!(pad_left("hello", 10, ' '), "     hello");
        assert_eq!(pad_left("hello", 3, ' '), "hello");
    }

    #[test]
    fn test_pad_right() {
        assert_eq!(pad_right("hello", 10, ' '), "hello     ");
        assert_eq!(pad_right("hello", 3, ' '), "hello");
    }
}
