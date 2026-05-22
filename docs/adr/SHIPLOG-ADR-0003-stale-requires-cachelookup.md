# SHIPLOG-ADR-0003: Stale Requires CacheLookup

Status: accepted
Date: 2026-05-13
Implementation status: implemented by
[`#244`](https://github.com/EffortlessMetrics/shiplog/pull/244)
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related specs:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
Related issue:
[#224](https://github.com/EffortlessMetrics/shiplog/issues/224)

## Context

`FreshnessStatus::Stale` exists in the schema crate and the intake-report
schema reserves the `stale` vocabulary. At the time of this decision, no
adapter could emit `stale` honestly.

The original SQLite cache lookup filtered expired rows out of `ApiCache::get`:

```sql
SELECT data FROM cache_entries WHERE key = ?1 AND expires_at > ?2
```

That means adapters see only:

```text
Some(value) = valid unexpired hit
None        = absent row or expired row
```

An adapter cannot honestly distinguish stale-hit from cache-miss from this API.
Emitting `stale` before the cache exposes that distinction would be false
precision.

## Decision

`FreshnessStatus::Stale` remains receipt-backed. It may be emitted only when
`ApiCache` exposes a lookup result that distinguishes fresh hit, stale hit, and
miss.

The accepted shape is:

```rust
enum CacheLookup<T> {
    Fresh(T),
    Stale(T),
    Miss,
}
```

Only `CacheLookup::Stale(_)` may produce a `stale` freshness receipt.

## Implementation Receipt

`CacheLookup::{Fresh, Stale, Miss}` landed in
[`#244`](https://github.com/EffortlessMetrics/shiplog/pull/244). The current
implementation keeps `ApiCache::get` compatible by returning only unexpired
rows and adds `ApiCache::lookup` for cache-aware adapters that need stale-hit
receipts.

The GitHub adapter uses `ApiCache::lookup` for JSON fetches, pull request
details, and review pages. Stale hits are recorded separately from fresh hits
and misses, and GitHub source freshness may now emit `stale` only after a
proven expired cache row is read through `CacheLookup::Stale(_)`.

## Consequences

- Reports may emit `fresh`, `cached`, `skipped`, `unavailable`, or `stale`.
- Reports must not emit `stale` by guessing from a cache miss.
- If an expired cache row exists, the source may be reported as `stale` only
  when the adapter received the expired value through `CacheLookup::Stale(_)`.
- Adding stale reporting is a cache/API behavior PR, not a schema wording
  cleanup.
- Tests for stale must seed an expired cache row and prove the adapter received
  a stale-hit receipt from the cache API.

## Alternatives Considered

### Infer Stale From A Miss After TTL

Rejected. A miss after TTL can mean an expired row, an absent row, a purged row,
or a lookup against a different key. The current API does not expose enough
information.

### Let Adapters Query Cache Internals Directly

Rejected. That would bypass `ApiCache` and spread SQLite expiry semantics into
adapters.

### Emit Stale As A Best-Effort Hint

Rejected. Freshness labels are receipts, not guesses. Users rely on them to
decide whether evidence is safe to trust and share.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
  records `stale` as reserved report vocabulary.
- [`SourceFreshness`](../../apps/shiplog/src/schema/freshness.rs) documents
  that `Stale` is emitted only from `CacheLookup::Stale(_)`.
- [`ApiCache::get`](../../apps/shiplog/src/cache/sqlite.rs) returns only
  unexpired hits.
- [`ApiCache::lookup`](../../apps/shiplog/src/cache/sqlite.rs) distinguishes
  fresh hits, stale hits, and misses.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  proves current first-run freshness behavior.
- GitHub adapter tests prove stale-hit freshness with an expired cache row and
  no network fetch.
