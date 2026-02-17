# shiplog-coverage

Date-window slicing helpers used by adaptive ingestion.

## API

- `month_windows(since, until)`
- `week_windows(since, until)`
- `day_windows(since, until)`
- `window_len_days(window)`

These utilities split an exclusive date range into smaller windows so ingestors can refine queries around provider caps.
