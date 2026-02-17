# shiplog (CLI)

`shiplog` is the command-line entrypoint for the workspace pipeline.

It wires ingestion, clustering, redaction, rendering, and bundling into one executable.

## Commands

- `collect`: ingest from a source and generate packet artifacts.
- `render`: re-render from existing `ledger.events.jsonl` + coverage/workstreams.
- `refresh`: re-fetch receipts while preserving workstream curation.
- `import`: ingest an existing run directory and render it.
- `run`: legacy one-shot mode (`collect` + `render`).

## Source adapters

- `github`: PR/review ingestion with adaptive slicing and optional cache.
- `json`: import canonical JSON artifacts.
- `manual`: ingest `manual_events.yaml`.

## Notes

- Curated `workstreams.yaml` is user-owned; generated suggestions go to `workstreams.suggested.yaml`.
- Manager/public packets are produced using deterministic redaction profiles.
- LLM clustering (`--llm-cluster`) requires building with `--features llm`.

## Example

```bash
cargo run -p shiplog -- collect github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --out ./out
```

See the repository `README.md` for full workflow details.
