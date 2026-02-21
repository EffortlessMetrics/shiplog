# shiplog-engine

Pipeline orchestration layer used by the CLI.

## Main types

- `Engine`: drives end-to-end flows.
- `RunOutputs`: output artifact paths for a run.
- `WorkstreamSource`: indicates whether workstreams were curated, suggested, or generated.

## Flows

- `run(...)`: ingest -> cluster/load workstreams -> render -> bundle.
- `import(...)`: render from imported ledger artifacts, optional imported workstreams.
- `refresh(...)`: update events/coverage while preserving existing workstream files.

`Engine` delegates behavior to the injected `Ingestor`, `WorkstreamClusterer`, `Renderer`, and `Redactor` implementations.
