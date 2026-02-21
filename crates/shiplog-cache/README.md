# shiplog-cache

SQLite-backed response cache used by GitHub ingestion.

## Key types

- `ApiCache`: generic JSON value cache with TTL expiration.
- `CacheKey`: stable key helpers for search/details/reviews endpoints.
- `CacheStats`: counts of total/valid/expired entries.

Cache entries are stored in SQLite and can be cleaned with `cleanup_expired()`.
