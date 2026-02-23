# shiplog-cache-stats

Canonical cache-stat normalization contracts for shiplog caches.

## Overview

`shiplog-cache-stats` isolates one responsibility from storage backends:
normalizing raw cache counters into stable, non-negative public stats.

This keeps stat-shape contracts reusable across cache implementations while
avoiding duplicate normalization logic.

## Usage

```rust
use shiplog_cache_stats::CacheStats;

let stats = CacheStats::from_raw_counts(42, 5, 9 * 1024 * 1024);
assert_eq!(stats.total_entries, 42);
assert_eq!(stats.expired_entries, 5);
assert_eq!(stats.valid_entries, 37);
assert_eq!(stats.cache_size_mb, 9);
```

## License

MIT OR Apache-2.0
