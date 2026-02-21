# shiplog-parse

Parsing utilities for shiplog.

## Overview

This crate provides parsing utilities for the shiplog ecosystem, including:
- Number parsing (u64, i64, f64)
- Boolean parsing
- Comma-separated list parsing
- Key-value pair parsing
- String trimming and unquoting

## Usage

```rust
use shiplog_parse::{parse_u64, parse_bool, parse_comma_separated};

let num = parse_u64("123").unwrap();
let flag = parse_bool("true").unwrap();
let items = parse_comma_separated("a,b,c");
```
