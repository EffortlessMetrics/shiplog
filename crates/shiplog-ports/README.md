# shiplog-ports

Core trait interfaces for shiplog's adapter boundaries.

## Traits

- `Ingestor`: returns canonical events plus coverage metadata.
- `WorkstreamClusterer`: groups events into `WorkstreamsFile`.
- `Renderer`: renders packet output from canonical data.
- `Redactor`: projects events/workstreams for profile-specific output.

Adapters (`shiplog-ingest-*`, renderers, clusterers, redactors) implement these traits.
