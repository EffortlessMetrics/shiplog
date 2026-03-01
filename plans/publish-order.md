# crates.io Publish Order

This document records the validated publish ordering for all workspace crates.
Crates must be published **leaf-first** (no unpublished dependencies).

> Generated from `cargo metadata` topological sort on the `feature/microcrate-extraction-v2` branch.

---

## Non-publishable crates

| Crate | Reason |
|---|---|
| `shiplog-testkit` | Test-only shared fixtures (`publish = false`) |
| `shiplog-fuzz` (in `fuzz/`) | Fuzz harnesses, not a workspace member (`publish = false`) |

---

## Publish tiers

### Tier 1 — Leaf crates (no workspace dependencies)

These 138 crates have zero workspace dependencies and can be published in any order
within the tier.

**Core leaf crates:**

| # | Crate | Description |
|---|---|---|
| 1 | `shiplog-ids` | Deterministic SHA-256 identifier types |
| 2 | `shiplog-output-layout` | Output directory layout conventions |
| 3 | `shiplog-cache-key` | Cache key computation |
| 4 | `shiplog-cache-stats` | Cache statistics types |
| 5 | `shiplog-cache-expiry` | Cache TTL / expiry logic |
| 6 | `shiplog-redaction-profile` | Redaction profile definitions |
| 7 | `shiplog-alias` | Deterministic alias generation |
| 8 | `shiplog-team-core` | Team aggregation core types |

**New utility / data-structure / pattern leaf crates (130 crates):**

All of the following are independent leaf crates with no workspace deps.
They can be published in any order within Tier 1.

<details>
<summary>Click to expand full list</summary>

`shiplog-accumulator`, `shiplog-actor`, `shiplog-adapter`, `shiplog-aggregator`,
`shiplog-archive`, `shiplog-async`, `shiplog-atomic`, `shiplog-auth`,
`shiplog-backup`, `shiplog-base64`, `shiplog-batcher`, `shiplog-bloom`,
`shiplog-bst`, `shiplog-btree`, `shiplog-bucket`, `shiplog-buffer`,
`shiplog-channel`, `shiplog-chars`, `shiplog-chunker`, `shiplog-circuit`,
`shiplog-circuitbreaker`, `shiplog-cogrouper`, `shiplog-collector`,
`shiplog-compressor`, `shiplog-config`, `shiplog-counter`, `shiplog-cron`,
`shiplog-crypto`, `shiplog-dedupe`, `shiplog-delayqueue`, `shiplog-deque`,
`shiplog-emitter`, `shiplog-enumerator`, `shiplog-env`, `shiplog-error`,
`shiplog-eventbus`, `shiplog-fenwick`, `shiplog-fmt`, `shiplog-gauge`,
`shiplog-graph`, `shiplog-guard`, `shiplog-hash`, `shiplog-health`,
`shiplog-heap`, `shiplog-hex`, `shiplog-histogram`, `shiplog-hooks`,
`shiplog-interceptor`, `shiplog-iter`, `shiplog-iterator`, `shiplog-joiner`,
`shiplog-latch`, `shiplog-leakybucket`, `shiplog-logging`, `shiplog-lru`,
`shiplog-matrix`, `shiplog-meter`, `shiplog-metrics`, `shiplog-middleware`,
`shiplog-migrate`, `shiplog-normalize`, `shiplog-normalizer`, `shiplog-notify`,
`shiplog-once`, `shiplog-parse`, `shiplog-path`, `shiplog-percentile`,
`shiplog-plugin`, `shiplog-pool`, `shiplog-priorityqueue`, `shiplog-prng`,
`shiplog-proc`, `shiplog-processor`, `shiplog-pubsub`, `shiplog-quantile`,
`shiplog-queue`, `shiplog-random`, `shiplog-rate`, `shiplog-reducer`,
`shiplog-regex`, `shiplog-report`, `shiplog-resolver`, `shiplog-retry`,
`shiplog-ring`, `shiplog-sanitize`, `shiplog-scheduler`, `shiplog-scope`,
`shiplog-segment`, `shiplog-semver`, `shiplog-serde`, `shiplog-shared`,
`shiplog-shutdown`, `shiplog-signal`, `shiplog-sink`, `shiplog-skiplist`,
`shiplog-slidingwindow`, `shiplog-spawn`, `shiplog-split`, `shiplog-stack`,
`shiplog-stats`, `shiplog-storage`, `shiplog-str`, `shiplog-stream`,
`shiplog-summary`, `shiplog-sync`, `shiplog-throttler`, `shiplog-time`,
`shiplog-timeout`, `shiplog-timewheel`, `shiplog-tracker`, `shiplog-transform`,
`shiplog-tree`, `shiplog-trie`, `shiplog-triggers`, `shiplog-ttl`,
`shiplog-tui`, `shiplog-union`, `shiplog-url`, `shiplog-uuid`,
`shiplog-validate`, `shiplog-validator`, `shiplog-version`, `shiplog-watch`,
`shiplog-watcher`, `shiplog-watermark`, `shiplog-web`, `shiplog-window`,
`shiplog-windowing`, `shiplog-workflow`, `shiplog-writer`

</details>

---

### Tier 2 — Depends only on Tier 1

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-schema` | `shiplog-ids` |
| 2 | `shiplog-cache-sqlite` | `shiplog-cache-expiry`, `shiplog-cache-key`, `shiplog-cache-stats` |
| 3 | `shiplog-export` | `shiplog-output-layout` |

---

### Tier 3 — Depends on Tier 1–2

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-bundle` | `shiplog-ids`, `shiplog-output-layout`, `shiplog-schema` |
| 2 | `shiplog-cache` | `shiplog-cache-sqlite` |
| 3 | `shiplog-cache-2q` | `shiplog-schema` |
| 4 | `shiplog-cache-arc` | `shiplog-schema` |
| 5 | `shiplog-cache-lru` | `shiplog-schema` |
| 6 | `shiplog-cache-ttl` | `shiplog-schema` |
| 7 | `shiplog-cluster-llm-parse` | `shiplog-ids`, `shiplog-schema` |
| 8 | `shiplog-cluster-llm-prompt` | `shiplog-schema` |
| 9 | `shiplog-date-windows` | `shiplog-schema` |
| 10 | `shiplog-diff` | `shiplog-ids`, `shiplog-schema` |
| 11 | `shiplog-filter` | `shiplog-ids`, `shiplog-schema` |
| 12 | `shiplog-manual-events` | `shiplog-ids`, `shiplog-schema` |
| 13 | `shiplog-ports` | `shiplog-schema` |
| 14 | `shiplog-receipt` | `shiplog-schema` |
| 15 | `shiplog-redaction-repo` | `shiplog-schema` |
| 16 | `shiplog-render-json` | `shiplog-schema` |
| 17 | `shiplog-template` | `shiplog-schema` |
| 18 | `shiplog-workstream-receipt-policy` | `shiplog-schema` |

---

### Tier 4 — Depends on Tier 1–3

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-coverage` | `shiplog-date-windows` |
| 2 | `shiplog-ingest-git` | `shiplog-cache`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 3 | `shiplog-ingest-json` | `shiplog-output-layout`, `shiplog-ports`, `shiplog-schema` |
| 4 | `shiplog-ingest-manual` | `shiplog-ids`, `shiplog-manual-events`, `shiplog-ports`, `shiplog-schema` |
| 5 | `shiplog-merge` | `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 6 | `shiplog-query` | `shiplog-filter`, `shiplog-ids`, `shiplog-schema` |
| 7 | `shiplog-redaction-policy` | `shiplog-redaction-profile`, `shiplog-redaction-repo`, `shiplog-schema` |
| 8 | `shiplog-render-md` | `shiplog-ports`, `shiplog-receipt`, `shiplog-schema`, `shiplog-workstream-receipt-policy` |
| 9 | `shiplog-team-render` | `shiplog-schema`, `shiplog-team-core`, `shiplog-template` |
| 10 | `shiplog-workstream-cluster` | `shiplog-ids`, `shiplog-ports`, `shiplog-schema`, `shiplog-workstream-receipt-policy` |
| 11 | `shiplog-workstream-layout` | `shiplog-ports`, `shiplog-schema` |

---

### Tier 5 — Depends on Tier 1–4

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-cluster-llm` | `shiplog-cluster-llm-parse`, `shiplog-cluster-llm-prompt`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema`, `shiplog-workstream-cluster` |
| 2 | `shiplog-ingest-github` | `shiplog-cache`, `shiplog-cache-key`, `shiplog-coverage`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 3 | `shiplog-ingest-gitlab` | `shiplog-cache`, `shiplog-cache-key`, `shiplog-coverage`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 4 | `shiplog-ingest-jira` | `shiplog-cache`, `shiplog-coverage`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 5 | `shiplog-ingest-linear` | `shiplog-cache`, `shiplog-coverage`, `shiplog-ids`, `shiplog-ports`, `shiplog-schema` |
| 6 | `shiplog-redaction-projector` | `shiplog-redaction-policy`, `shiplog-schema` |
| 7 | `shiplog-team-aggregate` | `shiplog-ids`, `shiplog-merge`, `shiplog-ports`, `shiplog-schema`, `shiplog-team-core`, `shiplog-team-render` |
| 8 | `shiplog-workstreams` | `shiplog-ids`, `shiplog-ports`, `shiplog-schema`, `shiplog-workstream-cluster`, `shiplog-workstream-layout` |

---

### Tier 6 — Depends on Tier 1–5

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-redact` | `shiplog-alias`, `shiplog-ports`, `shiplog-redaction-projector`, `shiplog-schema` |
| 2 | `shiplog-team` | `shiplog-team-aggregate` |

---

### Tier 7 — Depends on Tier 1–6

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog-engine` | `shiplog-bundle`, `shiplog-ids`, `shiplog-merge`, `shiplog-output-layout`, `shiplog-ports`, `shiplog-redact`, `shiplog-render-json`, `shiplog-render-md`, `shiplog-schema`, `shiplog-workstream-cluster`, `shiplog-workstream-layout` |

---

### Tier 8 — CLI application (final)

| # | Crate | Depends on |
|---|---|---|
| 1 | `shiplog` | `shiplog-cluster-llm`, `shiplog-engine`, `shiplog-ingest-git`, `shiplog-ingest-github`, `shiplog-ingest-json`, `shiplog-ingest-manual`, `shiplog-ports`, `shiplog-redact`, `shiplog-render-md`, `shiplog-schema`, `shiplog-team`, `shiplog-workstream-cluster`, `shiplog-workstreams` |

---

## Metadata checklist

All publishable crates inherit the following from `[workspace.package]`:

| Field | Value |
|---|---|
| `license` | `MIT OR Apache-2.0` |
| `repository` | `https://github.com/EffortlessMetrics/shiplog` |
| `homepage` | `https://effortlessmetrics.com/shiplog` |
| `edition` | `2024` |
| `rust-version` | `1.92` |

Each crate also provides its own:
- `description` — crate-specific ✓
- `readme` — `README.md` ✓
- `documentation` — `https://docs.rs/<crate-name>` ✓
- `keywords` — crate-specific ✓
- `categories` — crate-specific ✓

## Dry-run validation

```
$ cargo publish --dry-run --allow-dirty -p shiplog-ids
    Packaging shiplog-ids v0.2.1
    Verifying shiplog-ids v0.2.1
   Compiling shiplog-ids v0.2.1
    Finished `dev` profile
   Uploading shiplog-ids v0.2.1
warning: aborting upload due to dry run
```

## Notes

- `shiplog-testkit` is `publish = false` and is only used as a `[dev-dependencies]` entry,
  so it does not block publishing of any other crate.
- The `fuzz/` directory is a separate workspace (`publish = false`, not a workspace member).
- Tier 1 contains 138 leaf crates; within a tier, crates may be published in any order.
- Total publishable crates: **182**. Total tiers: **8**.
