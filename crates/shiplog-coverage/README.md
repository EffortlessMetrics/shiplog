# shiplog-coverage

Date-window utilities and completeness helpers used during ingestion.

## API

- `month_windows(since, until)`
- `week_windows(since, until)`
- `day_windows(since, until)`
- `window_len_days(window)`

These helpers support adaptive slicing when providers enforce result caps.
