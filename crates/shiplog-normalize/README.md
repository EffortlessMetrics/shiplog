# shiplog-normalize

Normalization utilities for shiplog.

## Overview

This crate provides normalization utilities for the shiplog ecosystem, including:
- String normalization (lowercase, trim)
- Path normalization (collapse slashes, remove trailing slashes)
- Whitespace normalization (collapse multiple spaces)
- Line ending normalization (convert to LF)
- Boolean normalization
- Number normalization (remove leading zeros, trailing decimal zeros)

## Usage

```rust
use shiplog_normalize::{normalize_string, normalize_path, normalize_whitespace};

let str = normalize_string("  HELLO  ");
let path = normalize_path("foo/bar//baz/");
let ws = normalize_whitespace("hello    world");
```
