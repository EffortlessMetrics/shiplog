# shiplog-sanitize

Sanitization utilities for shiplog.

## Overview

This crate provides sanitization utilities for the shiplog ecosystem, including:
- Control character removal (except whitespace)
- Non-ASCII character removal
- Filename sanitization
- Shell escaping
- Whitespace sanitization
- Null byte removal

## Usage

```rust
use shiplog_sanitize::{remove_control_characters, sanitize_filename, escape_shell};

let clean = remove_control_characters("hello\x00world");
let safe_name = sanitize_filename("file:name.txt");
let escaped = escape_shell("hello world");
```
