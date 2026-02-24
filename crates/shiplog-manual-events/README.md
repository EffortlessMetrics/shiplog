# shiplog-manual-events

Small, single-purpose utilities for the manual events workflow.

This microcrate owns:

- reading and writing `ManualEventsFile` YAML fixtures
- converting `ManualEventEntry` records to canonical `EventEnvelope`s
- filtering manual entries for a half-open `TimeWindow` with warning metadata

It is intentionally lightweight so it can be reused by ingest, tests, and future tooling.
