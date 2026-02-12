# shiplog

A **shipping packet generator** for developers.

It turns a date range of GitHub activity into:

- an **editable self‑review packet** (Markdown)
- an **evidence appendix** (JSONL)
- a **coverage manifest** that tells you what was captured, and what might be missing

This is not a productivity scoreboard.  
It is a **report compiler with an audit trail**.

## What it produces

Given a user + date range, `shiplog` generates a folder like:

```
out/<run_id>/
  packet.md
  workstreams.yaml
  ledger.events.jsonl
  coverage.manifest.json
  bundle.manifest.json
  profiles/
    manager/packet.md
    public/packet.md
```

Key design choice: **receipts first**.  
If the tool cannot prove it fetched something, it says so in `coverage.manifest.json`.

## Quick start

### GitHub mode

```bash
# token optional for public-only. required for private repos.
export GITHUB_TOKEN="ghp_..."

cargo run -p shiplog -- run github \
  --user octocat \
  --since 2025-01-01 \
  --until 2026-01-01 \
  --mode merged \
  --out ./out \
  --include-reviews
```

### JSON import mode

```bash
cargo run -p shiplog -- run json \
  --events ./examples/fixture/ledger.events.jsonl \
  --coverage ./examples/fixture/coverage.manifest.json \
  --out ./out
```

## Why this exists

Your devs do not need more PRs.  
They need fewer surprises per PR.

Performance reviews are the same shape: a year's worth of work, compressed into a handful
of workstreams with enough receipts that a reviewer can trust it without spelunking.

GitHub has the artifacts. It does not build the packet.

`shiplog` does.

## Architecture

This is a heavily microcrated Rust workspace with Clean Architecture boundaries:

- **crates/shiplog-schema**: canonical event model (the data spine)
- **crates/shiplog-ports**: traits (ingestors, renderers)
- **crates/shiplog-ingest-\***: adapters (GitHub, JSONL, future Jira/Linear)
- **crates/shiplog-engine**: orchestration (ingest → normalize → cluster → render)
- **crates/shiplog-workstreams**: clustering + editable YAML overrides
- **crates/shiplog-redact**: deterministic redaction profiles
- **crates/shiplog-coverage**: slicing + completeness reporting
- **crates/shiplog-render-\***: outputs (Markdown, JSON)
- **crates/shiplog-bundle**: checksums + optional zip export
- **apps/shiplog**: CLI

## Testing posture

- Unit tests in microcrates.
- Snapshot tests for rendered packets (`insta`).
- Property tests for redaction invariants (`proptest`).
- Fuzz harness scaffolding lives in `fuzz/` (not part of the workspace by default).

## Safety and trust

- Outputs include a **coverage manifest**.
- Redaction is deterministic and profile-based.
- Public packets strip titles and links by default. You should never leak private repo details.

## License

Dual-licensed under MIT or Apache-2.0.
