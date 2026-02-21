# shiplog-ingest-github

GitHub API ingestion adapter for PR and review activity.

## Main type

- `GithubIngestor`

## Behavior

- Collects authored PRs (`merged` or `created` mode).
- Optionally includes authored reviews (`include_reviews`).
- Uses adaptive month/week/day slicing to work around search caps and incomplete windows.
- Emits detailed `CoverageManifest` slices and partial-coverage notes when needed.
- Supports GitHub Enterprise API bases.
- Supports optional SQLite caching via `shiplog-cache`.
