# shiplog-path

Path handling utilities for shiplog.

## Usage

```rust
use shiplog_path::{normalize_path, join_paths, is_absolute_path};

let path = normalize_path("./foo/bar/../baz");
assert_eq!(path.to_string_lossy(), "foo/baz");

let joined = join_paths("/base", &["foo", "bar"]);
assert!(is_absolute_path(joined));
```
