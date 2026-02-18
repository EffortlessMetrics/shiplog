# shiplog-ingest-git

Local git repository ingestor for shiplog.

Collects commit history directly from local git repositories without requiring
GitHub API access or authentication.

## Usage

```rust
use shiplog_ingest_git::LocalGitIngestor;
use chrono::NaiveDate;

let ingestor = LocalGitIngestor::new(
    "/path/to/repo",
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
);

// Optional: filter by author email
let ingestor = ingestor.with_author("alice@example.com");

let output = ingestor.ingest()?;
```

## Features

- **Date filtering**: Collect commits within a specified date range
- **Author filtering**: Filter commits by author email
- **Branch awareness**: Track which branch commits were made on
- **Merge detection**: Identify merge commits and their relationships
- **No API required**: Works entirely with local git data

## Data Collected

For each commit, the ingestor creates an event with:

- Commit hash (as opaque_id)
- Commit message (as title)
- Author name and email
- Commit timestamp
- Repository name (from git config)
- Branch information (when available)
- Parent commit hashes (for merge detection)

## License

MIT OR Apache-2.0
