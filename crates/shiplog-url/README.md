# shiplog-url

URL parsing and utilities for shiplog.

## Usage

```rust
use shiplog_url::{parse_url, is_valid_url, get_host};

let url = parse_url("https://example.com/path?foo=bar").unwrap();
assert_eq!(get_host(&url), Some("example.com"));
assert!(is_valid_url("https://example.com"));
```
