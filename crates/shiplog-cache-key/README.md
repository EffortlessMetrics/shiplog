# shiplog-cache-key

Canonical cache-key construction helpers for shiplog API caches.

## Overview

`shiplog-cache-key` isolates key naming from cache storage implementations.
It provides stable key builders for:

- GitHub search requests
- GitHub pull-request details
- GitHub pull-request reviews
- GitLab merge-request notes

## Usage

```rust
use shiplog_cache_key::CacheKey;

let search_key = CacheKey::search("is:pr author:octocat", 1, 100);
let pr_key = CacheKey::pr_details("https://api.github.com/repos/o/r/pulls/1");
let review_key = CacheKey::pr_reviews("https://api.github.com/repos/o/r/pulls/1", 2);
let notes_key = CacheKey::mr_notes(42, 7, 1);
```

## License

MIT OR Apache-2.0
