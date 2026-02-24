# shiplog-date-windows

Date-window partition helpers split half-open date ranges into day, week, and month windows.

## API

- `day_windows(since, until)`
- `week_windows(since, until)`
- `month_windows(since, until)`
- `window_len_days(window)`

These functions are intended for windowing ingestion spans with stable boundary semantics:
- ranges are [since, until)
- windows are contiguous, non-overlapping, and cover exactly the input range
