# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-05-09

### Added

- Added the review rescue intake path: `shiplog intake` creates a best-effort
  first packet, records skipped sources as coverage warnings, renders the
  packet, runs review inspection, and writes durable `intake.report.md` and
  `intake.report.json` artifacts.
- Added intake readiness summaries, source-decision explanations, source repair
  and rerun guidance, curation-preserving intake reruns, and `shiplog open
  intake-report`.
- Added `shiplog review`, `shiplog review weekly`, `shiplog review --strict`,
  `shiplog review fixups`, `--commands-only`, and `--journal-template` for
  packet-quality inspection and copy-ready next actions.
- Added manual evidence capture and correction with `shiplog journal add`,
  `shiplog journal list`, and `shiplog journal edit`.
- Added manager/public share convenience commands and read-only share preflight
  checks with `shiplog share manager|public` and `shiplog share verify
  manager|public`.
- Added `shiplog share verify public --strict` to scan an existing or in-memory
  public packet for obvious raw URLs and original names before sharing.
- Added `shiplog runs compare` for read-only cross-run comparison.
- Added named review periods in `shiplog.toml` for `intake`, `collect multi`,
  and `review`.
- Added `shiplog identify jira` and `shiplog identify linear` to print provider
  account/user IDs for source configuration.
- Added `scripts/verify-release.sh` to repeat public post-release verification
  for GitHub assets, checksums, crates.io install, and binary smoke checks.

### Documentation

- Made the README quick start intake-first and added a review-deadline guide
  covering 5-minute, 15-minute, and 30-minute rescue flows.
- Added binary-first install documentation with release binary downloads,
  checksum verification, `cargo install`, and package-manager tracking notes.
- Documented v0.3.0 release verification and updated the current-state record
  from release-ready to shipped.
- Added a `shiplog.toml` config reference covering versioning, defaults,
  source fields, env vars, validation, and redaction safety.

### Fixed

- Changed CLI-generated packets to open with coverage and gaps before summary,
  workstreams, and receipts, and made packet mode default to a receipt-summary
  appendix while receipts mode keeps the full audit appendix.
- Polished Markdown packet summary labels so event counts pluralize correctly
  and source names use reader-facing labels.
- Fixed Jira search response deserialization for realistic REST payload field
  names.
- Fixed the packet file-artifacts section so it lists generated run artifacts
  instead of the manual-event input file.
- Fixed public redaction so provider-specific source opaque IDs are stripped
  along with source URLs.
- Fixed internal bundle zips so an archive written inside the run directory
  cannot include itself.
- Corrected redaction docs and package metadata to describe keyed SHA-256
  aliasing instead of HMAC.
- Improved share-profile redaction-key failures with copy-ready recovery
  guidance.

### Testing

- Added golden intake fixtures for manual-only rescue, all-source intake,
  skipped-source visibility, readiness output, manager-share missing-key
  safety, durable report content, curation-preserving reruns, and strict public
  verification.
- Added no-network provider edge smoke coverage for enterprise/self-hosted
  source configuration, invalid provider filters, partial source failures, and
  intake repair guidance.
- Added recorded-style GitHub provider payload tests for search, pull request
  details, and review parsing through cached event conversion.
- Added recorded-style GitLab provider payload tests for merge request and
  review-note parsing and event conversion.
- Added recorded-style Jira and Linear provider payload tests for adapter
  parsing and event conversion.
- Added a documented Codecov baseline and first informational project coverage
  target.
- Added the first documented mutation baseline for the `shiplog-coverage` trust
  surface.
- Added the documented mutation baseline for the `shiplog-ids` deterministic ID
  contract.
- Recorded the `shiplog-ports` mutation scan result as a no-target trait
  contract crate.
- Added the documented mutation baseline for the `shiplog-schema` persisted
  data contract.
- Added the documented mutation baseline for the `shiplog-redact` privacy trust
  surface.
- Added the documented mutation baseline for the `shiplog-bundle` artifact
  integrity surface.

### Security

- Kept manager/public sharing fail-closed while adding explicit share verbs,
  read-only preflight checks, and strict public verification guardrails that do
  not print redaction keys or token values.

## [0.3.0] - 2026-05-07

### Added

- Added CLI-supported GitLab, Jira, and Linear sources across `collect`, `refresh`, and legacy `run`.
- Added local git parity for `refresh git` and `run git`.
- Added `shiplog merge` to combine existing source runs into one packet.
- Added `shiplog collect multi` to collect enabled `shiplog.toml` sources into one merged packet while recording partial source failures in coverage warnings.
- Added first-run commands and defaults: `shiplog init`, `shiplog doctor`, `shiplog config validate`, `shiplog config explain`, `shiplog config migrate`, relative date presets, latest-run aliases, and GitHub/GitLab `--me` identity inference.
- Added run discovery and artifact opening with `shiplog runs list/show` and `shiplog open packet/workstreams/out`.
- Added workstream curation commands for list, validate, create, rename, move, split, receipt selection, and delete.
- Added packet coverage and limits summaries, source/gap summaries, evidence anchors, claim prompts, render modes (`packet`, `scaffold`, `receipts`), receipt limits, and appendix density controls.
- Added cache visibility and cleanup with `shiplog cache stats`, `shiplog cache inspect`, and `shiplog cache clean`.
- Added example configs and a review-cycle guide with fixture-safe docs command tests.

### Changed

- Promoted GitLab, Jira, Linear, team, and merge surfaces into the v0.3.0 publish set instead of leaving production-looking crates unpublished.
- Aligned the workspace package surface around publishable public crates plus dev-only tooling.
- Versioned `shiplog.toml` with `[shiplog] config_version = 1` while keeping legacy configs as implicit v1.
- Expanded release smoke tests to exercise product command help on the downloaded release artifact and release build.

### Security

- Manager and public render/bundle profiles now fail closed unless `--redact-key` or the configured redaction-key environment variable is provided.

### Testing and Release Proof

- Added package boundary and package version audits to prevent unpublished production crates and mixed release versions.
- Added fixture-safe command coverage for the review-cycle documentation path.
- Kept mutation testing advisory while the baseline matures.

## [0.2.1] - 2026-02-17

### Added

- CLI `manual` source wiring for `collect`, `refresh`, and `run`, enabling YAML manual-event ingestion through `shiplog-ingest-manual`
- GitHub CLI cache controls: `--cache-dir <PATH>` and `--no-cache`

### Changed

- GitHub ingestion now enables persistent API caching by default at `<out>/.cache` unless `--no-cache` is used
- `collect json` now honors `--regen` for suggested workstream regeneration, matching other collect sources
- `GithubIngestor::with_cache` now creates missing cache directories automatically

## [0.2.0] - 2026-02-15

### Added

- **LLM-assisted workstream clustering** (`shiplog-cluster-llm`): optional OpenAI-compatible endpoint for semantic workstream grouping, with automatic fallback to repo-based clustering on failure
- **`import` subcommand**: re-render a pre-built ledger directory from an upstream system or previous shiplog run
- **Redaction alias cache persistence**: `redaction.aliases.json` provides stable alias mappings across runs
- **`--zip` flag**: write a zip archive next to the run folder (available on `collect`, `render`, `refresh`, `import`, `run`)
- **`--run-dir` flag**: explicit run directory for `refresh` (overrides auto-detection)
- **`SourceSystem::Other(String)` variant**: extension point for third-party source systems (`#[non_exhaustive]`)
- **Bundle exclusion of `redaction.aliases.json`**: alias cache is excluded from bundle manifests and zip archives to prevent redaction bypass
- **LLM feature gate**: `shiplog-cluster-llm` is now an optional dependency behind the `llm` feature, off by default; build with `--features llm` to include it
- Module-level documentation (`//!` doc blocks) for `shiplog-schema`, `shiplog-coverage`, `shiplog-workstreams`, `shiplog-engine`, and `shiplog-ports`
- CLI Reference section in README.md with full flag table
- LLM clustering flags (`--llm-cluster`, `--llm-api-endpoint`, `--llm-model`, `--llm-api-key`) documented across all 4 doc files
- Missing sections in GEMINI.md and copilot-instructions.md to sync with CLAUDE.md

### Changed

- Crate-specific descriptions for all 15 publishable crates (replacing generic workspace description)
- Added `keywords` and `categories` to all publishable crate Cargo.toml files
- Marked `shiplog-testkit` as `publish = false`
- `CACHE_FILENAME` in `shiplog-redact` is now `pub` for cross-crate documentation

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
  - Deterministic keyed hash aliasing for repo names and workstream titles
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

[Unreleased]: https://github.com/EffortlessMetrics/shiplog/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/EffortlessMetrics/shiplog/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/EffortlessMetrics/shiplog/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/EffortlessMetrics/shiplog/compare/v0.0.1...v0.1.0
[0.0.1]: https://github.com/EffortlessMetrics/shiplog/releases/tag/v0.0.1
