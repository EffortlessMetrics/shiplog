# shiplog (CLI)

Command-line interface for the shiplog pipeline.

This binary wires together ingestion, clustering, redaction, rendering, and bundling from the workspace crates.

## Commands

- `collect`: fetch/import events, cluster workstreams, and render outputs.
- `render`: re-render from existing `ledger.events.jsonl` + workstreams.
- `refresh`: re-fetch events while preserving curated workstreams.
- `import`: ingest an existing run directory and re-render.
- `run`: one-shot legacy mode (`collect` + `render`).

## Example

```bash
cargo run -p shiplog -- collect github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --out ./out
```

For full end-to-end workflow details, see the repository `README.md`.
