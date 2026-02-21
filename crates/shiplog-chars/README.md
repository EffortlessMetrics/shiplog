# shiplog-chars

Character utilities for shiplog.

## Overview

This crate provides character manipulation utilities for the shiplog ecosystem.

## Features

- Vowel and consonant detection
- Digit validation (decimal, hex, octal)
- ASCII character operations
- Case conversion
- Character classification

## Usage

```rust
use shiplog_chars::{is_vowel, is_hex_digit, to_upper};

assert!(is_vowel('a'));
assert!(is_hex_digit('f'));
assert_eq!(to_upper('a'), 'A');
```
