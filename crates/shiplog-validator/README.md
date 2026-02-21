# shiplog-validator

Comprehensive validation utilities for shiplog data.

## Overview

This crate provides validators for various data types and formats used throughout the shiplog ecosystem, including email, URL, username, password, IP address, and numeric range validation.

## Usage

```rust
use shiplog_validator::{EmailValidator, UrlValidator, UsernameValidator, PasswordValidator};

fn main() {
    // Validate email
    assert!(EmailValidator::validate("user@example.com").is_ok());
    
    // Validate URL
    assert!(UrlValidator::validate("https://example.com").is_ok());
    
    // Validate username
    assert!(UsernameValidator::validate("john_doe").is_ok());
    
    // Validate password strength
    assert!(PasswordValidator::validate("StrongPass1").is_ok());
}
```

## Features

- **Email Validation**: RFC-compliant email format validation
- **URL Validation**: HTTP/HTTPS URL format validation
- **Username Validation**: Alphanumeric with underscore/hyphen support
- **Password Validation**: Strength checking with length and character requirements
- **IP Address Validation**: IPv4 address format and range validation
- **Range Validation**: Generic numeric range validation

## License

MIT OR Apache-2.0
