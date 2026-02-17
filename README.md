# shiplog

`shiplog` compiles activity data into self-review packets with receipts.

Given a user and date window, it produces:
- an editable Markdown packet,
- a canonical JSONL event ledger,
- a coverage manifest that documents what was queried and what might be incomplete.

## Output layout

```text
out/<run_id>/
  packet.md
  workstreams.yaml              # optional user-curated file
  workstreams.suggested.yaml    # generated suggestions
  ledger.events.jsonl
  coverage.manifest.json
  bundle.manifest.json
  profiles/
    manager/packet.md
    public/packet.md
```

`bundle.manifest.json` and optional zip output are profile-aware. `redaction.aliases.json` is intentionally excluded from bundles.

## Quick start

```bash
# 1) Collect from GitHub
cargo run -p shiplog -- collect github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out

# 2) Curate workstreams
# Rename workstreams.suggested.yaml -> workstreams.yaml and edit.

# 3) Re-render without re-fetching
cargo run -p shiplog -- render --out ./out
```

## Other flows

```bash
# Refresh receipts and coverage while preserving workstream curation
cargo run -p shiplog -- refresh github --user octocat --since 2025-01-01 --until 2026-01-01 --out ./out

# Import an existing ledger directory and re-render
cargo run -p shiplog -- import --dir ./path/to/run --out ./out

# Collect from prebuilt JSON artifacts
cargo run -p shiplog -- collect json --events ./ledger.events.jsonl --coverage ./coverage.manifest.json --out ./out

# Collect from manual YAML events
cargo run -p shiplog -- collect manual --events ./manual_events.yaml --user octocat --since 2025-01-01 --until 2026-01-01 --out ./out
```

## Redaction and profiles

Provide a redaction key with `--redact-key` or `SHIPLOG_REDACT_KEY` to generate stable manager/public projections:
- `internal`: no structural redaction,
- `manager`: keeps context, removes sensitive details,
- `public`: aliases repo/workstream names and strips sensitive fields.

## LLM clustering

Repository clustering is the default. To enable semantic clustering:

```bash
cargo run -p shiplog --features llm -- collect github ... --llm-cluster
```

`--llm-cluster` requires the `llm` feature and an API key (`--llm-api-key` or `SHIPLOG_LLM_API_KEY`).

## Workspace crates

- `apps/shiplog`: CLI entrypoint.
- `crates/shiplog-engine`: orchestration layer.
- `crates/shiplog-ports`: core traits.
- `crates/shiplog-schema`: canonical data model.
- `crates/shiplog-ids`: stable ID types.
- `crates/shiplog-ingest-*`: source adapters.
- `crates/shiplog-workstreams`: clustering plus curated/suggested YAML workflow.
- `crates/shiplog-redact`: deterministic profile-based redaction.
- `crates/shiplog-render-*`: Markdown and JSON artifact writers.
- `crates/shiplog-bundle`: checksum manifests and zip output.
- `crates/shiplog-cache`: SQLite API cache.
- `crates/shiplog-cluster-llm`: optional LLM clusterer.
- `crates/shiplog-testkit`: internal fixture/test helpers.

## License

Dual licensed under MIT OR Apache-2.0.
