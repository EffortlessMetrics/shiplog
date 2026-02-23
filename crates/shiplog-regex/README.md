# shiplog-regex

Regex utilities for shiplog.

## Overview

This crate provides regex utilities for the shiplog ecosystem.

## Features

- Pattern compilation and validation
- Match detection
- Find all matches
- Find and replace
- Capture group extraction
- String splitting by pattern

## Usage

```rust
use shiplog_regex::{is_match, find_all, replace_all};

assert!(is_match(r"\d+", "123").unwrap());

let matches = find_all(r"\d+", "123 abc 456").unwrap();
assert_eq!(matches, vec!["123", "456"]);

let result = replace_all(r"\d+", "abc123def", "X").unwrap();
assert_eq!(result, "abcXdef");
```
