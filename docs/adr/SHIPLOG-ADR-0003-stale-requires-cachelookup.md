# SHIPLOG-ADR-0003: Stale Requires CacheLookup

Status: accepted
Date: 2026-05-13
Related proposal:
[`SHIPLOG-PROP-0001-user-polish-release`](../proposals/SHIPLOG-PROP-0001-user-polish-release.md)
Related specs:
[`SHIPLOG-SPEC-0002-intake-report-v1`](../specs/SHIPLOG-SPEC-0002-intake-report-v1.md)
Related issue:
[#224](https://github.com/EffortlessMetrics/shiplog/issues/224)

## Context

`FreshnessStatus::Stale` exists in the schema crate and the intake-report
schema reserves the `stale` vocabulary. No adapter emits `stale` today.

The current SQLite cache lookup filters expired rows out of `ApiCache::get`:

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

`FreshnessStatus::Stale` remains reserved but unemitted until `ApiCache` exposes
a lookup result that distinguishes fresh hit, stale hit, and miss.

The intended shape is:

```rust
enum CacheLookup<T> {
    Fresh(T),
    Stale(T),
    Miss,
}
```

Only `CacheLookup::Stale(_)` may produce a `stale` freshness receipt.

## Consequences

- Current reports may emit `fresh`, `cached`, `skipped`, or `unavailable`.
- Current reports must not emit `stale`.
- If an expired cache row exists but the live fetch succeeds, the source may be
  reported according to the adapter's proven fresh/cached behavior, not as
  stale.
- If an expired cache row exists and the live fetch fails, the source remains
  `unavailable` unless the adapter received the expired value through
  `CacheLookup::Stale(_)`.
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
- [`SourceFreshness`](../../crates/shiplog-schema/src/freshness.rs) documents
  that `Stale` is reserved until `CacheLookup` exists.
- [`ApiCache::get`](../../crates/shiplog-cache/src/sqlite.rs) currently returns
  only unexpired hits.
- [`apps/shiplog/tests/intake_cold_start.rs`](../../apps/shiplog/tests/intake_cold_start.rs)
  proves current first-run freshness behavior.
- The future CacheLookup implementation PR should update cache tests, adapter
  tests, and the user-polish plan proof commands together.
