# shiplog-team-core

Reusable team mode primitives for shiplog:

- `TeamConfig` model and YAML loading.
- Deterministic, deduplicated parsing for CSV-ish member and alias lists.
- `resolve_team_config` for normalizing CLI/config combinations.

This crate intentionally contains the pure, stable config contract for team aggregation.
`shiplog-team` composes this with merge/render/output behavior.
