# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-02-14

### Added

- Module-level documentation (`//!` doc blocks) for `shiplog-schema`, `shiplog-coverage`, `shiplog-workstreams`, and `shiplog-engine`
- CLI Reference section in README.md with full flag table
- Missing sections in GEMINI.md and copilot-instructions.md to sync with CLAUDE.md (CLI flags, error handling, runtime, BDD testing, output directory structure)

### Changed

- Crate-specific descriptions for all 15 publishable crates (replacing generic workspace description)
- Added `keywords` and `categories` to all publishable crate Cargo.toml files
- Marked `shiplog-testkit` as `publish = false`

## [0.1.1] - 2026-02-14

### Changed

- Refactored MarkdownRenderer for improved readability and consistency
- Enhanced documentation in CLAUDE.md with error handling, runtime, and output directory details
- Added package metadata (description, repository) for crates.io publishing
- Fixed internal crate dependencies to specify version requirements for publishing

## [0.1.0] - 2026-02-12

### Added

- **Core Ports and Traits** (`shiplog-ports`):
  - `Ingestor` trait for data collection adapters
  - `Renderer` trait for output format generation
  - `Redactor` trait for privacy-aware output filtering
  - `WorkstreamClusterer` trait for event clustering algorithms

- **GitHub Ingestor** (`shiplog-ingest-github`):
  - Fetch PRs and reviews from GitHub API
  - Adaptive date slicing to handle GitHub's 1000-result search cap
  - Support for both "merged" and "created" PR modes
  - Throttling support for rate limit compliance
  - GHES (GitHub Enterprise Server) support via custom API base
  - **SQLite caching** for PR details and reviews to reduce API calls

- **JSON Ingestor** (`shiplog-ingest-json`):
  - Import events from JSONL files
  - Coverage manifest validation

- **Manual Events** (`shiplog-ingest-manual`):
  - Track non-GitHub work (incidents, design docs, mentoring, launches, migrations)
  - YAML-based manual event definitions
  - Event type classification with emoji support

- **Local SQLite Cache** (`shiplog-cache`):
  - Durable caching for GitHub API responses
  - TTL-based expiration (default 24 hours)
  - Cache key builder for GitHub endpoints
  - In-memory cache support for testing
  - Cache statistics and cleanup utilities

- **Workstream Clustering** (`shiplog-workstreams`):
  - Repository-based automatic clustering
  - Curated workstreams via `workstreams.yaml`
  - Suggested workstreams generation (`workstreams.suggested.yaml`)
  - Persistent workstream management (user edits preserved)
  - Manager for curation workflow

- **Redaction System** (`shiplog-redact`):
  - Three redaction profiles: `internal`, `manager`, `public`
  - Deterministic HMAC-based aliasing for repo names and workstream titles
  - Per-field redaction rules:
    - Public: strips titles, URLs, paths, descriptions
    - Manager: keeps titles/repos but removes sensitive details
    - Internal: no redaction
  - Property-based testing for leak detection

- **Markdown Renderer** (`shiplog-render-md`):
  - Self-review packet generation in Markdown
  - Coverage summary with completeness tracking
  - Event counts by type (PRs, reviews, manual)
  - Query slicing details and warnings
  - Receipt truncation with appendix for full listing
  - Claim scaffolds for narrative writing

- **JSON Renderer** (`shiplog-render-json`):
  - JSON output format for programmatic consumption

- **Bundle Format** (`shiplog-bundle`):
  - Zip archive generation for distribution
  - Manifest with integrity verification
  - Structured packet organization

- **Engine** (`shiplog-engine`):
  - `collect` command: Fetch events and generate workstream suggestions
  - `render` command: Regenerate packets from existing data
  - `refresh` command: Update events while preserving workstream curation
  - `run` command: Legacy combined collect+render mode

- **Schema** (`shiplog-schema`):
  - Event envelopes with unique IDs
  - Event types: PullRequest, Review, Manual
  - Coverage manifests with slicing metadata
  - Workstream definitions with receipts and stats
  - Manual event types and classification

- **IDs** (`shiplog-ids`):
  - Type-safe ID generation (EventId, RunId, WorkstreamId)
  - Timestamp-based run identifiers

- **Coverage** (`shiplog-coverage`):
  - Time window utilities (day, week, month windows)
  - Completeness tracking (Complete, Partial)
  - Coverage slicing for API cap handling

- **Testing** (`shiplog-testkit`):
  - Fixture generators for property-based tests
  - Redaction leak detection utilities

### Changed

- Enhanced `ApiCache` with `Clone` and `Debug` implementations
- Added `Serialize` derive to GitHub API response structs for cache storage
- Cleaned up all compiler warnings across the workspace

## [0.0.1] - Initial Release

### Added

- Initial project structure
- Basic workspace configuration with Cargo
- MIT/Apache-2.0 dual licensing

[Unreleased]: https://github.com/EffortlessMetrics/shiplog/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.0.1
