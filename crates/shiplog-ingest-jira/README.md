# shiplog-ingest-jira

Jira issue/ticket ingestor with API integration.

## Overview

This crate provides an ingestor for collecting issue and ticket events from Jira. It integrates with the shiplog pipeline to provide:

- Issue collection with status filtering
- Support for Jira Cloud and self-hosted instances
- Coverage tracking with partial completeness detection
- Optional SQLite cache for API responses

## Usage

```rust
use shiplog_ingest_jira::JiraIngestor;
use chrono::NaiveDate;

let ingestor = JiraIngestor::new(
    "alice".to_string(),
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
)
.with_token("your-jira-token".to_string())?
.with_instance("your-company.atlassian.net".to_string())?;

let output = ingestor.ingest()?;
```

## Configuration

- `user`: Jira username or email to collect issues for
- `since`: Start date for collection
- `until`: End date for collection
- `token`: Jira API token (required)
- `instance`: Jira instance URL (default: "jira.atlassian.com")
- `status`: Filter issues by status (open, in_progress, done, etc.)
- `throttle_ms`: Delay between API requests for rate limiting
- `cache`: Optional API cache

## API Endpoints

The ingestor uses the Jira REST API v3:

- `/rest/api/3/search` - Search for issues
- `/rest/api/3/issue/{key}` - Get issue details

## Error Handling

The ingestor returns detailed errors for:
- Invalid Jira instance URLs
- Authentication failures
- Rate limit exceeded
- Network issues

## License

MIT OR Apache-2.0
