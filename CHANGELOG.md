# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-02-11

### Added

- **Local SQLite Cache** (`shiplog-cache`): New crate providing durable caching for GitHub API responses to reduce API calls and speed up repeated runs
  - `ApiCache` with TTL-based expiration (default 24 hours)
  - Cache key builder for GitHub endpoints (`CacheKey::pr_details()`, `CacheKey::pr_reviews()`)
  - In-memory cache support for testing
  - Cache statistics and cleanup utilities

- **GitHub Ingestor Cache Integration**: `shiplog-ingest-github` now caches PR details and reviews
  - `GithubIngestor::with_cache()` for file-based caching
  - `GithubIngestor::with_in_memory_cache()` for testing
  - Automatic cache lookup before API calls
  - Transparent cache storage after fetches

- **Release Preparation**: Cleaned up all compiler warnings across the workspace for a clean release build

### Changed

- Enhanced `ApiCache` with `Clone` and `Debug` implementations for better integration
- Added `Serialize` derive to GitHub API response structs for cache storage

## [0.1.0-alpha] - Prior to 2025-02-11

### Added

- Initial shiplog implementation with core functionality:
  - **GitHub Ingestor** (`shiplog-ingest-github`): Fetch PRs and reviews with adaptive date slicing to handle GitHub's 1000-result cap
  - **JSON Ingestor** (`shiplog-ingest-json`): Import from JSONL event files
  - **Manual Events** (`shiplog-ingest-manual`): Track non-GitHub work (incidents, design docs, mentoring, etc.)
  
- **Workstream Clustering** (`shiplog-workstreams`): 
  - Repo-based clustering algorithm
  - Curated workstreams via `workstreams.yaml`
  - Suggested workstreams for user editing
  - Persistent workstream management (no auto-overwrite)

- **Redaction System** (`shiplog-redact`):
  - Three redaction profiles: `internal`, `manager`, `public`
  - Deterministic aliasing for repo names and workstream titles
  - Property-based testing for leak detection
  - Per-field redaction rules (titles, URLs, descriptions, paths)

- **Markdown Renderer** (`shiplog-render-md`):
  - Self-review packet generation
  - Coverage summary with completeness tracking
  - Receipt truncation with appendix
  - Claim scaffolds for narrative writing

- **Engine** (`shiplog-engine`):
  - `collect` command: Fetch events and generate workstream suggestions
  - `render` command: Regenerate packets from existing data
  - `refresh` command: Update events while preserving curation
  - `run` command: Legacy combined mode

- **Bundle Format** (`shiplog-bundle`):
  - Zip archive generation
  - Manifest with integrity verification
  - Structured packet organization

- **Schema** (`shiplog-schema`):
  - Event envelopes with unique IDs
  - Coverage manifests with slicing metadata
  - Workstream definitions

- **Testing** (`shiplog-testkit`):
  - Fixture generators for property-based tests
  - Redaction leak detection

[Unreleased]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.1.0
