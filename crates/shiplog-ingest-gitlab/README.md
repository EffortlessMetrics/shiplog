# shiplog-ingest-gitlab

GitLab MR/review ingestor with adaptive date slicing and optional SQLite cache.

## Overview

This crate provides an ingestor for collecting merge request and review events from GitLab (both gitlab.com and self-hosted instances). It integrates with the shiplog pipeline to provide:

- Merge request collection with state filtering (opened, merged, closed)
- Review event collection
- Self-hosted GitLab instance support
- Adaptive date slicing for efficient API usage
- Optional SQLite cache for API responses
- Coverage tracking with partial completeness detection

## Usage

```rust
use shiplog_ingest_gitlab::GitlabIngestor;
use chrono::NaiveDate;

let ingestor = GitlabIngestor::new(
    "alice".to_string(),
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
)
.with_token("glpat_xxxxxx".to_string())?
.with_instance("gitlab.com".to_string())?
.with_include_reviews(true)?;

let output = ingestor.ingest()?;
```

## Configuration

- `user`: GitLab username to collect MRs/reviews for
- `since`: Start date for collection
- `until`: End date for collection
- `token`: GitLab personal access token (required)
- `instance`: GitLab instance URL (default: "gitlab.com")
- `include_reviews`: Whether to collect review events
- `state`: Filter MRs by state (opened, merged, closed, all)
- `throttle_ms`: Delay between API requests for rate limiting
- `cache`: Optional API cache

## API Endpoints

The ingestor uses the GitLab REST API v4:

- `/projects/:id/merge_requests` - List merge requests
- `/projects/:id/merge_requests/:iid/notes` - List MR notes (for reviews)

## Error Handling

The ingestor returns detailed errors for:
- Invalid GitLab instance URLs
- Authentication failures
- Rate limit exceeded
- Private/inaccessible projects
- Network issues

## License

MIT OR Apache-2.0
