## Release v0.1.0

This PR prepares the shiplog project for its v0.1.0 release, introducing the SQLite cache integration and hardening the workspace for production use.

### 🚀 New Features

#### Local SQLite Cache (`shiplog-cache`)
- **Durable caching** for GitHub API responses to reduce API calls and speed up repeated runs
- **TTL-based expiration** (default 24 hours) with configurable per-entry TTL
- **Cache key builder** for GitHub endpoints (`CacheKey::pr_details()`, `CacheKey::pr_reviews()`)
- **In-memory cache support** for testing environments
- **Cache statistics and cleanup utilities** (`stats()`, `cleanup_expired()`)

#### GitHub Ingestor Cache Integration
- `GithubIngestor::with_cache()` - Enable file-based SQLite caching
- `GithubIngestor::with_in_memory_cache()` - Enable in-memory caching for tests
- **Automatic cache lookup** before API calls for PR details and reviews
- **Transparent cache storage** after fetches with automatic serialization

### 🔧 Technical Improvements

#### Serde Derive Hardening
- Fixed serde import issues using fully-qualified derives (`serde::Serialize`, `serde::Deserialize`)
- Eliminated "unused import" warnings in test modules
- Proper `Serialize` derives on all cached DTOs (`PullRequestDetails`, `PullRequestReview`, etc.)

#### Warning Cleanup
- Fixed doc comment on proptest macro (converted to regular comment)
- **Zero compiler warnings** across entire workspace
- Clean `cargo clippy --workspace` run

#### ApiCache Enhancements
- Added `Clone` implementation for `GithubIngestor` derive compatibility
- Added `Debug` derive for better debugging experience

### 📋 Changelog

See [CHANGELOG.md](./CHANGELOG.md) for detailed version history:
- **v0.0.1**: Initial project structure
- **v0.1.0**: All features including caching, redaction, workstreams, and rendering

### ✅ Release Checklist

- [x] All 29 tests passing
- [x] Zero compiler warnings
- [x] SQLite cache integration complete
- [x] CHANGELOG.md updated
- [x] Workspace compiles cleanly

### 🧪 Test Results

```
cargo test --workspace
# 29 tests passed, 0 failed

shiplog-cache:       6 tests ✓
shiplog-coverage:    2 tests ✓
shiplog-ingest-manual: 3 tests ✓
shiplog-redact:     12 tests ✓
shiplog-render-md:   2 tests ✓
shiplog-workstreams: 4 tests ✓
```

### 🏗️ Architecture

```
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│  shiplog-cache  │────▶│  SQLite DB   │     │   GitHub API    │
│  (new crate)    │     │  (TTL cache) │◀────│  (when needed)  │
└─────────────────┘     └──────────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────┐     ┌─────────────────┐
│shiplog-ingest-  │────▶│  PR Details  │     │  PR Reviews     │
│    github       │     │  (cached)    │     │  (cached)       │
└─────────────────┘     └──────────────┘     └─────────────────┘
```

### 📦 Crates in this Release

| Crate | Version | Description |
|-------|---------|-------------|
| `shiplog` | 0.1.0 | Main CLI application |
| `shiplog-cache` | 0.1.0 | **NEW**: SQLite API response caching |
| `shiplog-ingest-github` | 0.1.0 | GitHub ingestor with cache integration |
| `shiplog-redact` | 0.1.0 | Deterministic redaction system |
| `shiplog-workstreams` | 0.1.0 | Workstream clustering |
| `shiplog-render-md` | 0.1.0 | Markdown packet renderer |
| `shiplog-engine` | 0.1.0 | Core engine (collect/render/refresh) |
| `shiplog-ports` | 0.1.0 | Port traits |
| `shiplog-schema` | 0.1.0 | Event/coverage/workstream schemas |
| + 6 more crates | 0.1.0 | Supporting infrastructure |

---

**Ready for review and merge to main for v0.1.0 tag.**
