//! Comprehensive validation utilities for shiplog data.
//!
//! This crate provides validators for various data types and formats
//! used throughout the shiplog ecosystem.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Result type for validation operations.
pub type ValidatorResult<T> = Result<T, ValidatorError>;

/// Validation error with field and message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorError {
    pub field: String,
    pub message: String,
    pub code: ErrorCode,
}

impl fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.code, self.field, self.message)
    }
}

impl std::error::Error for ValidatorError {}

/// Error codes for validation failures.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    Empty,
    InvalidFormat,
    OutOfRange,
    InvalidValue,
    TooLong,
    TooShort,
    Missing,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::Empty => write!(f, "EMPTY"),
            ErrorCode::InvalidFormat => write!(f, "INVALID_FORMAT"),
            ErrorCode::OutOfRange => write!(f, "OUT_OF_RANGE"),
            ErrorCode::InvalidValue => write!(f, "INVALID_VALUE"),
            ErrorCode::TooLong => write!(f, "TOO_LONG"),
            ErrorCode::TooShort => write!(f, "TOO_SHORT"),
            ErrorCode::Missing => write!(f, "MISSING"),
        }
    }
}

/// Email validator
pub struct EmailValidator;

impl EmailValidator {
    /// Validates an email address format.
    pub fn validate(email: &str) -> ValidatorResult<()> {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap();

        if email.is_empty() {
            return Err(ValidatorError {
                field: "email".to_string(),
                message: "email cannot be empty".to_string(),
                code: ErrorCode::Empty,
            });
        }

        if !email_regex.is_match(email) {
            return Err(ValidatorError {
                field: "email".to_string(),
                message: format!("invalid email format: {}", email),
                code: ErrorCode::InvalidFormat,
            });
        }

        Ok(())
    }
}

/// URL validator
pub struct UrlValidator;

impl UrlValidator {
    /// Validates a URL format.
    pub fn validate(url: &str) -> ValidatorResult<()> {
        let url_regex = Regex::new(r"^https?://[^\s/$.?#].[^\s]*$").unwrap();

        if url.is_empty() {
            return Err(ValidatorError {
                field: "url".to_string(),
                message: "URL cannot be empty".to_string(),
                code: ErrorCode::Empty,
            });
        }

        if !url_regex.is_match(url) {
            return Err(ValidatorError {
                field: "url".to_string(),
                message: format!("invalid URL format: {}", url),
                code: ErrorCode::InvalidFormat,
            });
        }

        Ok(())
    }
}

/// Username validator
pub struct UsernameValidator;

impl UsernameValidator {
    const MIN_LENGTH: usize = 3;
    const MAX_LENGTH: usize = 32;

    /// Validates a username format.
    pub fn validate(username: &str) -> ValidatorResult<()> {
        if username.is_empty() {
            return Err(ValidatorError {
                field: "username".to_string(),
                message: "username cannot be empty".to_string(),
                code: ErrorCode::Empty,
            });
        }

        if username.len() < Self::MIN_LENGTH {
            return Err(ValidatorError {
                field: "username".to_string(),
                message: format!("username must be at least {} characters", Self::MIN_LENGTH),
                code: ErrorCode::TooShort,
            });
        }

        if username.len() > Self::MAX_LENGTH {
            return Err(ValidatorError {
                field: "username".to_string(),
                message: format!("username must be at most {} characters", Self::MAX_LENGTH),
                code: ErrorCode::TooLong,
            });
        }

        let username_regex = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
        if !username_regex.is_match(username) {
            return Err(ValidatorError {
                field: "username".to_string(),
                message: "username can only contain letters, numbers, underscores, and hyphens"
                    .to_string(),
                code: ErrorCode::InvalidFormat,
            });
        }

        Ok(())
    }
}

/// Password strength validator
pub struct PasswordValidator;

impl PasswordValidator {
    const MIN_LENGTH: usize = 8;

    /// Validates password strength.
    pub fn validate(password: &str) -> ValidatorResult<()> {
        if password.is_empty() {
            return Err(ValidatorError {
                field: "password".to_string(),
                message: "password cannot be empty".to_string(),
                code: ErrorCode::Empty,
            });
        }

        if password.len() < Self::MIN_LENGTH {
            return Err(ValidatorError {
                field: "password".to_string(),
                message: format!("password must be at least {} characters", Self::MIN_LENGTH),
                code: ErrorCode::TooShort,
            });
        }

        let has_upper = password.chars().any(|c| c.is_uppercase());
        let has_lower = password.chars().any(|c| c.is_lowercase());
        let has_digit = password.chars().any(|c| c.is_ascii_digit());

        if !has_upper || !has_lower || !has_digit {
            return Err(ValidatorError {
                field: "password".to_string(),
                message: "password must contain uppercase, lowercase, and digit".to_string(),
                code: ErrorCode::InvalidValue,
            });
        }

        Ok(())
    }
}

/// IP address validator
pub struct IpAddressValidator;

impl IpAddressValidator {
    /// Validates an IPv4 address format.
    pub fn validate_ipv4(ip: &str) -> ValidatorResult<()> {
        let parts: Vec<&str> = ip.split('.').collect();

        if parts.len() != 4 {
            return Err(ValidatorError {
                field: "ip_address".to_string(),
                message: "IPv4 must have 4 octets".to_string(),
                code: ErrorCode::InvalidFormat,
            });
        }

        for part in parts {
            let octet: u32 = part.parse().map_err(|_| ValidatorError {
                field: "ip_address".to_string(),
                message: format!("invalid octet: {}", part),
                code: ErrorCode::InvalidFormat,
            })?;

            if octet > 255 {
                return Err(ValidatorError {
                    field: "ip_address".to_string(),
                    message: format!("octet out of range: {}", octet),
                    code: ErrorCode::OutOfRange,
                });
            }
        }

        Ok(())
    }
}

/// Numeric range validator
pub struct RangeValidator<T: PartialOrd> {
    min: Option<T>,
    max: Option<T>,
}

impl<T: PartialOrd + Copy + std::fmt::Display> RangeValidator<T> {
    /// Create a new range validator.
    pub fn new(min: Option<T>, max: Option<T>) -> Self {
        Self { min, max }
    }

    /// Validate a value is within range.
    pub fn validate(&self, value: T) -> ValidatorResult<()> {
        if let Some(min) = self.min
            && value < min
        {
            return Err(ValidatorError {
                field: "value".to_string(),
                message: format!("value {} is less than minimum {}", value, min),
                code: ErrorCode::OutOfRange,
            });
        }

        if let Some(max) = self.max
            && value > max
        {
            return Err(ValidatorError {
                field: "value".to_string(),
                message: format!("value {} is greater than maximum {}", value, max),
                code: ErrorCode::OutOfRange,
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_valid() {
        assert!(EmailValidator::validate("test@example.com").is_ok());
        assert!(EmailValidator::validate("user.name+tag@domain.co.uk").is_ok());
    }

    #[test]
    fn test_email_invalid() {
        assert!(EmailValidator::validate("").is_err());
        assert!(EmailValidator::validate("invalid").is_err());
        assert!(EmailValidator::validate("@domain.com").is_err());
    }

    #[test]
    fn test_url_valid() {
        assert!(UrlValidator::validate("https://example.com").is_ok());
        assert!(UrlValidator::validate("http://localhost:8080").is_ok());
    }

    #[test]
    fn test_url_invalid() {
        assert!(UrlValidator::validate("").is_err());
        assert!(UrlValidator::validate("not a url").is_err());
    }

    #[test]
    fn test_username_valid() {
        assert!(UsernameValidator::validate("john_doe").is_ok());
        assert!(UsernameValidator::validate("user123").is_ok());
    }

    #[test]
    fn test_username_invalid() {
        assert!(UsernameValidator::validate("").is_err());
        assert!(UsernameValidator::validate("ab").is_err());
        assert!(UsernameValidator::validate("user@name").is_err());
    }

    #[test]
    fn test_password_valid() {
        assert!(PasswordValidator::validate("StrongPass1").is_ok());
        assert!(PasswordValidator::validate("MyP@ssw0rd").is_ok());
    }

    #[test]
    fn test_password_invalid() {
        assert!(PasswordValidator::validate("").is_err());
        assert!(PasswordValidator::validate("weak").is_err());
        assert!(PasswordValidator::validate("alllowercase1").is_err());
    }

    #[test]
    fn test_ipv4_valid() {
        assert!(IpAddressValidator::validate_ipv4("192.168.1.1").is_ok());
        assert!(IpAddressValidator::validate_ipv4("10.0.0.1").is_ok());
        assert!(IpAddressValidator::validate_ipv4("255.255.255.255").is_ok());
    }

    #[test]
    fn test_ipv4_invalid() {
        assert!(IpAddressValidator::validate_ipv4("").is_err());
        assert!(IpAddressValidator::validate_ipv4("192.168.1").is_err());
        assert!(IpAddressValidator::validate_ipv4("192.168.1.256").is_err());
    }

    #[test]
    fn test_range_validator() {
        let validator = RangeValidator::new(Some(0), Some(100));

        assert!(validator.validate(50).is_ok());
        assert!(validator.validate(0).is_ok());
        assert!(validator.validate(100).is_ok());

        assert!(validator.validate(-1).is_err());
        assert!(validator.validate(101).is_err());
    }

    #[test]
    fn test_error_display() {
        let error = ValidatorError {
            field: "email".to_string(),
            message: "invalid format".to_string(),
            code: ErrorCode::InvalidFormat,
        };

        assert_eq!(
            format!("{}", error),
            "[INVALID_FORMAT] email: invalid format"
        );
    }
}
