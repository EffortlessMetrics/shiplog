//! Character utilities for shiplog.
//!
//! This crate provides character manipulation utilities for the shiplog ecosystem.

/// Checks if a character is a vowel (a, e, i, o, u)
pub fn is_vowel(c: char) -> bool {
    matches!(c.to_ascii_lowercase(), 'a' | 'e' | 'i' | 'o' | 'u')
}

/// Checks if a character is a consonant
pub fn is_consonant(c: char) -> bool {
    c.is_alphabetic() && !is_vowel(c)
}

/// Checks if a character is a digit (0-9)
pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Checks if a character is a hex digit (0-9, a-f, A-F)
pub fn is_hex_digit(c: char) -> bool {
    c.is_ascii_hexdigit()
}

/// Checks if a character is an octal digit (0-7)
pub fn is_octal_digit(c: char) -> bool {
    matches!(c, '0'..='7')
}

/// Checks if a character is a punctuation mark
pub fn is_punctuation(c: char) -> bool {
    c.is_ascii_punctuation()
}

/// Checks if a character is whitespace (space, tab, newline, etc.)
pub fn is_whitespace(c: char) -> bool {
    c.is_whitespace()
}

/// Checks if a character is uppercase
pub fn is_uppercase(c: char) -> bool {
    c.is_uppercase()
}

/// Checks if a character is lowercase
pub fn is_lowercase(c: char) -> bool {
    c.is_lowercase()
}

/// Converts a character to uppercase
pub fn to_upper(c: char) -> char {
    c.to_ascii_uppercase()
}

/// Converts a character to lowercase
pub fn to_lower(c: char) -> char {
    c.to_ascii_lowercase()
}

/// Checks if a character is an ASCII character
pub fn is_ascii(c: char) -> bool {
    c.is_ascii()
}

/// Checks if a character is alphanumeric (letter or digit)
pub fn is_alphanumeric(c: char) -> bool {
    c.is_alphanumeric()
}

/// Checks if a character is a letter (alphabetic)
pub fn is_letter(c: char) -> bool {
    c.is_alphabetic()
}

/// Gets the ASCII value of a character
pub fn ascii_value(c: char) -> u8 {
    c as u8
}

/// Creates an ASCII character from a value
pub fn from_ascii(value: u8) -> char {
    value as char
}

/// Checks if a character is a control character (ASCII 0-31 or 127)
pub fn is_control(c: char) -> bool {
    c.is_ascii_control()
}

/// Checks if a character is printable (not a control character)
pub fn is_printable(c: char) -> bool {
    c.is_ascii_graphic() || c == ' '
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vowel() {
        assert!(is_vowel('a'));
        assert!(is_vowel('E'));
        assert!(is_vowel('I'));
        assert!(!is_vowel('b'));
        assert!(!is_vowel('z'));
    }

    #[test]
    fn test_is_consonant() {
        assert!(is_consonant('b'));
        assert!(is_consonant('Z'));
        assert!(!is_consonant('a'));
        assert!(!is_consonant('1'));
    }

    #[test]
    fn test_is_digit() {
        assert!(is_digit('0'));
        assert!(is_digit('9'));
        assert!(!is_digit('a'));
        assert!(!is_digit('-'));
    }

    #[test]
    fn test_is_hex_digit() {
        assert!(is_hex_digit('0'));
        assert!(is_hex_digit('9'));
        assert!(is_hex_digit('a'));
        assert!(is_hex_digit('f'));
        assert!(is_hex_digit('A'));
        assert!(is_hex_digit('F'));
        assert!(!is_hex_digit('g'));
    }

    #[test]
    fn test_is_octal_digit() {
        assert!(is_octal_digit('0'));
        assert!(is_octal_digit('7'));
        assert!(!is_octal_digit('8'));
        assert!(!is_octal_digit('9'));
    }

    #[test]
    fn test_is_punctuation() {
        assert!(is_punctuation('!'));
        assert!(is_punctuation('.'));
        assert!(is_punctuation(','));
        assert!(!is_punctuation('a'));
        assert!(!is_punctuation('1'));
    }

    #[test]
    fn test_is_whitespace() {
        assert!(is_whitespace(' '));
        assert!(is_whitespace('\t'));
        assert!(is_whitespace('\n'));
        assert!(!is_whitespace('a'));
    }

    #[test]
    fn test_is_uppercase() {
        assert!(is_uppercase('A'));
        assert!(is_uppercase('Z'));
        assert!(!is_uppercase('a'));
    }

    #[test]
    fn test_is_lowercase() {
        assert!(is_lowercase('a'));
        assert!(is_lowercase('z'));
        assert!(!is_lowercase('A'));
    }

    #[test]
    fn test_to_upper() {
        assert_eq!(to_upper('a'), 'A');
        assert_eq!(to_upper('Z'), 'Z');
    }

    #[test]
    fn test_to_lower() {
        assert_eq!(to_lower('A'), 'a');
        assert_eq!(to_lower('z'), 'z');
    }

    #[test]
    fn test_is_ascii() {
        assert!(is_ascii('a'));
        assert!(is_ascii('Z'));
        assert!(!is_ascii('é'));
    }

    #[test]
    fn test_is_alphanumeric() {
        assert!(is_alphanumeric('a'));
        assert!(is_alphanumeric('5'));
        assert!(!is_alphanumeric('!'));
    }

    #[test]
    fn test_is_letter() {
        assert!(is_letter('a'));
        assert!(is_letter('Z'));
        assert!(!is_letter('5'));
    }

    #[test]
    fn test_ascii_value() {
        assert_eq!(ascii_value('A'), 65);
        assert_eq!(ascii_value('a'), 97);
        assert_eq!(ascii_value('0'), 48);
    }

    #[test]
    fn test_from_ascii() {
        assert_eq!(from_ascii(65), 'A');
        assert_eq!(from_ascii(97), 'a');
    }

    #[test]
    fn test_is_control() {
        assert!(is_control('\0'));
        assert!(is_control('\n'));
        assert!(!is_control('a'));
    }

    #[test]
    fn test_is_printable() {
        assert!(is_printable('a'));
        assert!(is_printable(' '));
        assert!(!is_printable('\n'));
    }
}
