# Clippy Protected Field Seams

This document defines the **protected field seams** that `clippy::disallowed_fields` will eventually enforce. The lint itself is currently `[[planned]]` in [`policy/clippy-lints.toml`](../policy/clippy-lints.toml) — held until the seams documented here are scaffolded (now landed at [`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml)) and at least one seam has been refactored to use accessors (planned for the next PR).

This is doc-only. **No lint is activated by this document.** It exists to anchor the design conversation before any of the downstream activations land.

## Why protected fields

The `disallowed_fields` lint bans **direct field access** on selected types. The point is not to forbid struct fields generally — most fields are fine to read directly. The point is to enforce that certain *load-bearing* fields can only be touched through an accessor that maintains an invariant.

In shiplog, every protected seam is something that has historically caused, or can plausibly cause, a class of bug where:

- A reader assumes the field's raw value is safe to expose.
- A writer assumes the field can be overwritten in isolation.
- A refactor renames or reshapes the field and downstream surfaces silently desync.

The accessor layer documents the invariant once and forces every caller through it.

## Protected field classes

Each class below lists:

- **Invariant** — what the field carries that the accessor protects.
- **Boundary** — which module / crate owns the accessor.
- **Today's surface** — where the field actually lives in the current codebase (or expected to be added; flagged when uncertain).
- **Failure mode without the accessor** — the kind of regression the lint catches.

### 1. Redaction internals

| Slot | Value |
| --- | --- |
| Invariant | Alias maps, key material, and profile state must never be exposed in raw form. Internal name → alias mappings stay private; lookups go through the accessor that returns the alias plus the redaction profile it was minted under. |
| Boundary | `shiplog-redact` crate. Public types expose accessors (`alias_for`, `profile_of_alias`, etc.); raw `aliases: HashMap<…>` / `key_material: Vec<u8>` style fields are private. |
| Today's surface | `shiplog-redact::*` — alias caches, HMAC-SHA256 key state, profile descriptor structs. |
| Failure mode | A renderer reads `redactor.aliases` directly and emits the inner real name to a manager- or public-profile bundle. The redaction profile leak goes unnoticed because no test exercises the raw read path. |

### 2. Bundle paths

| Slot | Value |
| --- | --- |
| Invariant | Paths recorded in `bundle.manifest.json` and `share.manifest.json` must be relative to the run directory, must not include the run-dir prefix itself, and must never include absolute paths from the producer's filesystem. Accessors normalise on write and validate on read. |
| Boundary | `shiplog-bundle` crate. `BundleManifest` / `ShareManifest` expose `entries()` returning normalised relative paths; raw `path: PathBuf` style fields are private. |
| Today's surface | `shiplog-bundle::*` — manifest entries, file checksum receipts, zip-archive path lists. |
| Failure mode | A test uses a producer-local absolute path; the path leaks into the manifest; another machine cannot verify the bundle because the path is invalid there. |

### 3. Trust receipts

| Slot | Value |
| --- | --- |
| Invariant | Hashes, signatures, and provenance receipts written to `intake.report.json`, `share.manifest.json`, and `bundle.manifest.json` must be computed via the canonical accessor on the source struct. Two callers that recompute the same receipt must produce byte-identical bytes. |
| Boundary | The receipt-emitting crate (varies by receipt type): `shiplog-engine` for intake reports, `shiplog-bundle` for bundle/share manifests, `shiplog-cache` for cache receipts. Each owns the canonical `compute_receipt()` accessor; raw `hash: [u8; 32]` style fields are private. |
| Today's surface | Mixed: `shiplog-engine::intake_report`, `shiplog-bundle::manifest`, `shiplog-cache::receipt` (some types here may need to be introduced rather than refactored). |
| Failure mode | One caller hashes the canonical fields; a second caller hashes a slightly different field order; the two receipts disagree for the same logical input and consumers diverge. |

### 4. Source opaque IDs

| Slot | Value |
| --- | --- |
| Invariant | Provider-specific identifiers (GitHub `node_id`, GitLab project IDs, Jira issue keys, Linear UUIDs, opaque `local_git` ref names) must be aliased before they cross a redaction profile boundary. Accessors return the aliased form for non-internal profiles and the raw form only when an `Internal` profile is explicitly requested. |
| Boundary | `shiplog-ingest-*` crates own the raw IDs (deserialised from API responses); `shiplog-schema` owns the canonical wrapped types; `shiplog-redact` is the only consumer that touches the raw form, and only to mint the alias. |
| Today's surface | The `*Event` types in `shiplog-schema` plus the per-source raw response structs in `shiplog-ingest-{github,gitlab,jira,linear,git,manual}`. The opaque-ID fields are currently `pub` in many of these structs (event flow predates this lint), so the refactor work in #191 is genuine, not just an audit. |
| Failure mode | A public-profile render reads `event.github_node_id` directly; the rendered packet leaks the GitHub-internal `node_id` that should have been aliased. |

### 5. Cache internals

| Slot | Value |
| --- | --- |
| Invariant | The SQLite-backed `shiplog-cache` exposes a query API; the raw `rusqlite::Connection`, the schema-version row, and the cache-key construction layout must be private. Cache key construction goes through `CacheKey::new(...)` which centralises the cache-namespace + version + hash algorithm. |
| Boundary | `shiplog-cache` crate. Public surface: `Cache::get`, `Cache::put`, `Cache::stats`, etc. Private: `Connection`, `schema_version`, `CacheKey::raw`. |
| Today's surface | `shiplog-cache::*` — already mostly private; the lint protects the boundary from regressing as new accessors are added. |
| Failure mode | A new ingest adapter borrows `cache.connection` directly to run a custom SQL query; the cache schema migrates in a future release; the custom query breaks silently. |

### 6. Policy ledger metadata

| Slot | Value |
| --- | --- |
| Invariant | Every `policy/*.toml` ledger carries a common header (`schema_version`, `policy`, `owner`, `status`). The header lives in a shared `Ledger<Body>` wrapper in `xtask::policy`; downstream consumers (the `cargo xtask check-*` checkers) access the body via the wrapper's accessor, never by destructuring the raw `toml::Table`. |
| Boundary | `xtask::policy` module (in the `xtask` crate). The wrapper struct owns the header validation; checkers read `ledger.body()` and never reach for `ledger.raw_table`. |
| Today's surface | `xtask::policy::*`. The wrapper exists implicitly today (each checker re-parses the header); a future refactor consolidates it. |
| Failure mode | A new checker forgets to validate the common header and silently accepts an unversioned ledger. Drift in the header schema becomes invisible because individual checkers don't enforce it consistently. |

## Activation ladder

`disallowed_fields` is not activated by this document. The planned sequence:

1. **PR #188**: wrote this document; cross-linked from `policy/clippy-lints.toml` (the existing `[[planned]]` entry's `reason` field). No lint config, no Rust code change.
2. **PR #190**: scaffolded `policy/clippy-protected-fields.toml` with six `[[class]]` entries (one per class above). `cargo xtask check-policy-schemas` accepts the new ledger; lint remains `[[planned]]`.
3. **This PR**: cache-internals accessor pass. Verified by call-site grep that no caller outside `crates/shiplog-cache/src/sqlite.rs` reaches into `ApiCache::{conn, default_ttl, max_size_bytes}`; advanced the ledger entry `cpf-0005` from `activation_status = "scaffolded"` to `"accessor-ready"`; added a rustdoc citation on `ApiCache` so future contributors do not introduce a raw-connection escape hatch. No `clippy.toml` change.
4. **Next PR**: activate `clippy::disallowed_fields` for cache-internals only (`clippy.toml` lists the three `ApiCache` fields from `cpf-0005`). Run full clippy/test/doc gate. No existing production callers reach the raw fields, so the activation is expected not to require entries in `policy/clippy-exceptions.toml`.
5. **Subsequent PRs**: repeat the accessor pass + per-class activation for the remaining five classes in approximate refactor-size order (bundle-paths, policy-ledger-metadata, redaction-internals, trust-receipts, source-opaque-ids).

The constraint at every step: **never activate the lint without a working accessor surface**. A direct activation against the current surface would produce 100+ findings on day one, drown the operator in exceptions, and either ship broken or grind to a halt. The ladder makes the protection real one class at a time.

## Out of scope

- **Test-side carveouts.** Per the operating contract, tests do not get blanket Clippy carveouts. Where a test legitimately reaches into a protected field (e.g. asserting an alias map state), use `#[expect(clippy::disallowed_fields, reason = "policy:test/...")]` with a policy ID, not a `#[allow(...)]`.
- **`pub(crate)` vs accessor.** Some seams may end up as `pub(crate)` rather than full accessors — that's fine for boundaries that don't need a normalising step, as long as the lint config and `clippy-protected-fields.toml` match what was actually chosen.
- **External types.** Foreign struct fields (from `serde_json::Value`, `rusqlite::Row`, etc.) are not protected by this lint. The boundary is at the shiplog crate edge.

## See also

- [`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml) — the scaffolded ledger that encodes the six classes above per-entry (id, slug, invariant, boundary, accessor pattern, field candidates, activation status).
- [`policy/clippy-lints.toml`](../policy/clippy-lints.toml) — the current Clippy lint ledger, where `clippy::disallowed_fields` lives as a `[[planned]]` entry until the protected-fields scaffold lands.
- [`docs/CLIPPY_POLICY.md`](CLIPPY_POLICY.md) — overall Clippy doctrine, lint ratchets, suppression style.
- [`docs/POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — the common header schema every policy ledger shares (the "policy ledger metadata" seam above).
