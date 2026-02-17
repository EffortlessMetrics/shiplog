# shiplog-engine

Pipeline orchestration layer for shiplog.

`Engine` coordinates ingestion, clustering, redaction, rendering, and bundle output for CLI operations.

## Key types

- `Engine`
- `RunOutputs`
- `WorkstreamSource`

Use this crate to run pipeline flows without depending on CLI argument parsing.
