# shiplog-ingest-github

GitHub API ingestion adapter.

## Features

- Collects PR events (and optional review events) for a user/date range.
- Uses adaptive month/week/day slicing around API result caps.
- Emits detailed `CoverageManifest` slices and warnings.
- Supports GitHub Enterprise API base URLs.
- Supports optional SQLite caching via `shiplog-cache`.

Primary type: `GithubIngestor`.
