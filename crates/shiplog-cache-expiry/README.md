# shiplog-cache-expiry

Canonical cache-expiry helpers for shiplog cache implementations.

This crate has one responsibility: representing cache timestamp windows and
normalizing expiry checks (`expires_at > now` for validity).

## What It Provides

- `CacheExpiryWindow`: `cached_at` + `expires_at` timestamps
- `is_valid` / `is_expired`: canonical expiry predicates
- `now_rfc3339` and RFC3339 parsing helpers

## Example

```rust
use chrono::Duration;
use shiplog_cache_expiry::{CacheExpiryWindow, is_valid};

let window = CacheExpiryWindow::from_now(Duration::hours(24));
let still_valid = is_valid(window.expires_at, window.cached_at);
assert!(still_valid);
```
