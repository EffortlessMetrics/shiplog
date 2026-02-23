# shiplog-normalizer

Data normalization utilities for shiplog.

## Overview

This crate provides data transformation and normalization utilities with a focus on structured data formats like JSON, YAML, CSV, and other common data types.

## Usage

```rust
use shiplog_normalizer::{normalize_slug, normalize_version, normalize_json_key};

fn main() {
    // Convert names to slugs
    assert_eq!(normalize_slug("Hello World"), "hello-world");
    
    // Normalize versions
    assert_eq!(normalize_version("v1.5"), "1.5.0");
    
    // Normalize JSON keys
    assert_eq!(normalize_json_key("camelCase"), "camel_case");
}
```

## Features

- **JSON Key Normalization**: Convert camelCase, PascalCase to snake_case
- **Slug Generation**: Create URL-friendly slugs from names
- **Version Normalization**: Canonicalize version strings to semver format
- **CSV Header Normalization**: Standardize CSV column names
- **Phone Number Normalization**: Convert to E.164 format
- **Configurable String Normalizer**: Customizable normalization pipeline

## License

MIT OR Apache-2.0
