# shiplog-cache

SQLite-backed cache for API responses.

## Key types

- `ApiCache`: read/write JSON payloads with TTL.
- `CacheKey`: stable cache key builders for GitHub endpoints.
- `CacheStats`: cache metrics snapshot.

Designed to reduce repeated API calls and improve reproducibility in ingest workflows.
