# shiplog-ingest-linear

Linear issue/ticket ingestor with API integration.

This crate provides functionality to ingest issues from Linear API and convert them to shiplog events.

## Features

- Issue ingestion from Linear API
- Filtering by project, status, and assignee
- Coverage tracking with partial completeness detection
- Optional caching for API responses
- Throttling support for rate limit compliance

## Usage

```rust
use shiplog_ingest_linear::LinearIngestor;
use chrono::NaiveDate;

let ingestor = LinearIngestor::new(
    "alice".to_string(),
    NaiveDate::from_ymd_opt(2025, 1, 1).unwrap(),
    NaiveDate::from_ymd_opt(2025, 2, 1).unwrap(),
)
.with_api_key("your-linear-api-key".to_string())?
.with_project("PROJ-123".to_string());

let output = ingestor.ingest()?;
```

## Configuration

- `api_key`: Linear API key (required)
- `project`: Optional project filter
- `status`: Optional status filter (Backlog, Todo, In Progress, Done, Cancelled, All)
- `throttle_ms`: Delay between API requests in milliseconds (default: 0)
- `cache`: Optional API response cache

## License

MIT OR Apache-2.0
