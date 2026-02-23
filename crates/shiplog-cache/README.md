# shiplog-cache

Local SQLite cache for GitHub API responses.

Stores JSON responses in SQLite with configurable TTL and thread-safe connection management.

## Features

- **TTL Configuration**: Set default TTL for cache entries (24 hours by default, configurable via `with_ttl()` method
- **Cache Size Limits**: Set maximum cache size limit to prevent unbounded growth
- **Cache Inspection Commands**: View cache statistics including total entries, valid entries, expired entries, and cache size on disk
- **Cache Cleanup**: Clear all entries or clean up expired entries
- **Thread-Safe**: Internal connection management ensures thread safety

## Usage

```rust
use shiplog_cache::ApiCache;

// Open or create cache at the given path
let cache = ApiCache::open("./.shiplog-cache.db")?;

// Set a value with default TTL (24 hours)
cache.set("key", serde_json::json!({"value": "data"}))?;

// Set a value with custom TTL (7 days)
cache.set_with_ttl("key", serde_json::json!({"value": "data"}), std::time::Duration::from_secs(7 * 24 * 60 * 60))?;

// Get a value (returns None if expired or not found)
let value: Option<String> = cache.get("key")?;

// Store a value with default TTL
cache.set("key", serde_json::json!({"value": "data"}))?;

// Get cache statistics
let stats = cache.stats()?;
println!("Total entries: {}", stats.total_entries);
println!("Valid entries: {}", stats.valid_entries);
println!("Expired entries: {}", stats.expired_entries);
println!("Cache size: {} MB", stats.cache_size_mb);
```

## API

### `ApiCache`

#### `new(path: impl AsRef<Path>) -> Result<Self>`

Open or create cache at the given path. If the database doesn't exist, it will be created with the schema.

```rust
let cache = ApiCache::open("./.shiplog-cache.db")?;
```

#### `with_max_size(max_size_bytes: u64) -> Self`

Create a cache with a maximum size limit.

```rust
let cache = ApiCache::open("./.cache.db")?.with_max_size(1024 * 1024 * 1024)?;
```

#### `with_ttl(ttl: Duration) -> Self`

Set the default TTL for cache entries.

```rust
let cache = ApiCache::open("./cache.db")?.with_ttl(std::time::Duration::from_secs(7 * 24 * 60 * 60))?;
```

### `get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>>`

Get a cached value if it exists and hasn't expired. Returns `None` if the key doesn't exist or the entry has expired.

```rust
let value: Option<String> = cache.get("key")?;
```

### `set<T: Serialize>(&self, key: &str, value: &T) -> Result<()>`

Store a value in the cache with the default TTL.

```rust
cache.set("key", serde_json::json!({"value": "data"}))?;
```

### `set_with_ttl<T: Serialize>(&self, key: &str, value: &T, ttl: Duration) -> Result<()>`

Store a value in the cache with a custom TTL.

```rust
cache.set_with_ttl("key", serde_json::json!({"value": "data"}), std::time::Duration::from_secs(7 * 24 * 60 * 60))?;
```

### `contains(&self, key: &str) -> Result<bool>`

Check if a key exists and hasn't expired. Returns `false` if the key doesn't exist or the entry has expired.

```rust
let exists: bool = cache.contains("key")?;
```

### `cleanup_expired(&self) -> Result<usize>`

Remove expired entries from the cache. Returns the number of deleted entries.

```rust
let deleted = cache.cleanup_expired()?;
```

### `clear(&self) -> Result<()>`

Clear all entries from the cache.

```rust
cache.clear()?;
```

### `stats(&self) -> Result<CacheStats>`

Get cache statistics including total entries, valid entries, expired entries, and cache size on disk.

```rust
let stats = cache.stats()?;
```

## Cache Statistics

The `CacheStats` struct includes:

- `total_entries: usize` - Total number of entries in the cache
- `valid_entries: usize` - Number of entries that haven't expired
- `expired_entries: usize` - Number of entries that have expired
- `cache_size_mb: u64` - Cache size in megabytes

## Thread Safety

The cache uses internal connection management with `Arc` for thread-safe sharing across multiple connections.
