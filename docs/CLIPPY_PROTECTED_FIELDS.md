# Clippy Protected Field Seams

This document defines the **protected field seams** that `clippy::disallowed_fields` was originally scoped to enforce. The lint remains `[[planned]]` in [`policy/clippy-lints.toml`](../policy/clippy-lints.toml) — intentionally, not as backlog. See [Outcome (as of #216)](#outcome-as-of-216) for the per-class result and the doctrine the ladder converged on.

This is doc-only. **No lint is activated by this document.** The classes below remain the canonical reference for the seams and the failure modes they guard against; the per-class audit history (with dated findings) lives in [`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml).

## Outcome (as of #216)

The six-class protected-fields ladder is complete for the current codebase. Every class evaluated through the ladder reached `"type-enforced"` — except `cpf-0002` bundle-paths, where the reachable surface (`RunArtifactPaths::out_dir`) was tightened to `pub(crate)` in [#207](https://github.com/EffortlessMetrics/shiplog/pull/207) and the remaining work waits for manifest entry types that do not yet exist in the codebase.

| Class | Status | Protection mechanism |
|---|---|---|
| `cpf-0001` redaction-internals | `type-enforced` | Internal alias-store type is `pub(crate)`; structural privacy from day one. |
| `cpf-0002` bundle-paths | `scaffolded` | `RunArtifactPaths::out_dir` tightened to `pub(crate)` in #207; manifest entry types do not yet exist. |
| `cpf-0003` trust-receipts | `type-enforced` | Single canonical computer in `shiplog::bundle` + determinism property tests + fuzz harness. |
| `cpf-0004` source-opaque-ids | `type-enforced` | Canonical `SourceRef::opaque_id` wrapper + procedural nullification in `shiplog::redact::policy` + integration tests. |
| `cpf-0005` cache-internals | `type-enforced` | Inner-struct refactor in #194 made raw fields unreachable; `ApiCacheInner` is private. |
| `cpf-0006` policy-ledger-metadata | `type-enforced` | `xtask` is `publish = false` (crate-boundary) + dedicated `check-policy-schemas` gate (procedural). |

### Doctrine

The ladder converged on three rules for choosing a protection mechanism:

1. **Prefer structure / type enforcement.** Make the type unreachable — private inner struct, `pub(crate)` wrapper, crate `publish = false`. When the type cannot be named externally, the field cannot be read by path; the lint is moot.
2. **Use `clippy::disallowed_fields` only when protected raw fields must remain reachable on a `pub` type.** Once a class takes the inner-struct route, the lint emits non-suppressible `"does not refer to a reachable field"` warnings against the now-nonexistent paths (see `cpf-0005` activation-probe history). The lint is a tier-2 tool, applicable only when the type's `pub` shape is non-negotiable (e.g. serde schema requirement) — and even then, procedural protection (canonical computer + determinism tests) often fits the failure mode better.
3. **Use `xtask` only for semantic / cross-file invariants.** A future ledger-driven `xtask check-protected-fields` checker could catch real cross-file shapes (e.g. a new `pub fn raw_*` accessor on a protected boundary type) — not patterns Clippy can already catch within a single file.

### Do not re-open

- **`clippy::disallowed_fields` activation.** No class currently needs it activated. The entry stays `[[planned]]` against the possibility that a future class keeps protected fields on a `pub` type. Re-running an activation probe against the current codebase will re-derive the same `"does not refer to a reachable field"` finding recorded in `cpf-0005`.
- **Per-class audits.** Every class has a dated audit-history block in `policy/clippy-protected-fields.toml`. The audits are the receipt; do not re-run them without a code change that actually shifts a class's posture.

### Open follow-ups

These are real but **not** currently in flight; do not pre-emptively pick them up:

- **`cpf-0002` bundle-paths.** Waits for the bundle / share manifest entry types to land. When they do, apply the inner-struct pattern from day one rather than retrofitting it post hoc (the cache-internals lesson from #192 → #194).
- **`LoadedPolicy` visibility tightening.** Optional `pub` → `pub(crate)` tightening on `xtask::policy::LoadedPolicy::{path, header, raw}`. The `cpf-0006` audit established this would not break any within-crate caller. Mirrors #208's `RunArtifactPaths::out_dir` change.
- **Ledger-driven `xtask check-protected-fields`.** Optional step 6 in the activation ladder. Only justify it when a future regression demonstrates a real cross-file shape Clippy cannot catch — speculative scaffolding is out of scope.

### Final audit PR

[#216](https://github.com/EffortlessMetrics/shiplog/pull/216) closed the loop: `cpf-0004` source-opaque-ids advanced to `type-enforced`, completing the six-class ladder. Each class's audit-history block in `policy/clippy-protected-fields.toml` carries the dated finding and the PR that recorded it.

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
| Boundary | `shiplog::redact` module. Public types expose accessors (`alias_for`, `profile_of_alias`, etc.); raw `aliases: HashMap<…>` / `key_material: Vec<u8>` style fields are private. |
| Today's surface | `shiplog::redact::*` — alias caches, HMAC-SHA256 key state, profile descriptor structs. |
| Failure mode | A renderer reads `redactor.aliases` directly and emits the inner real name to a manager- or public-profile bundle. The redaction profile leak goes unnoticed because no test exercises the raw read path. |

### 2. Bundle paths

| Slot | Value |
| --- | --- |
| Invariant | Paths recorded in `bundle.manifest.json` and `share.manifest.json` must be relative to the run directory, must not include the run-dir prefix itself, and must never include absolute paths from the producer's filesystem. Accessors normalise on write and validate on read. |
| Boundary | `shiplog::bundle` module. `BundleManifest` / `ShareManifest` expose `entries()` returning normalised relative paths; raw `path: PathBuf` style fields are private. |
| Today's surface | `shiplog::bundle::*` — manifest entries, file checksum receipts, zip-archive path lists. |
| Failure mode | A test uses a producer-local absolute path; the path leaks into the manifest; another machine cannot verify the bundle because the path is invalid there. |

### 3. Trust receipts

| Slot | Value |
| --- | --- |
| Invariant | Hashes, signatures, and provenance receipts written to `intake.report.json`, `share.manifest.json`, and `bundle.manifest.json` must be computed via the canonical accessor on the source struct. Two callers that recompute the same receipt must produce byte-identical bytes. |
| Boundary | The receipt-emitting owner module (varies by receipt type): `shiplog::engine` for intake reports, `shiplog::bundle` for bundle/share manifests, `shiplog::cache` for cache receipts. Each owns the canonical `compute_receipt()` accessor; raw `hash: [u8; 32]` style fields are private. |
| Today's surface | Mixed: `shiplog::engine::intake_report`, `shiplog::bundle::manifest`, `shiplog::cache::receipt` (some types here may need to be introduced rather than refactored). |
| Failure mode | One caller hashes the canonical fields; a second caller hashes a slightly different field order; the two receipts disagree for the same logical input and consumers diverge. |

### 4. Source opaque IDs

| Slot | Value |
| --- | --- |
| Invariant | Provider-specific identifiers (GitHub `node_id`, GitLab project IDs, Jira issue keys, Linear UUIDs, opaque `local_git` ref names) must be aliased before they cross a redaction profile boundary. Accessors return the aliased form for non-internal profiles and the raw form only when an `Internal` profile is explicitly requested. |
| Boundary | `shiplog::ingest::*` modules own the raw IDs (deserialised from API responses); `shiplog::schema` owns the canonical wrapped types; `shiplog::redact` is the only consumer that touches the raw form, and only to mint the alias. |
| Today's surface | The `*Event` types in `shiplog::schema` plus the per-source raw response structs in `shiplog::ingest::{github,gitlab,jira,linear,git,manual}`. The opaque-ID fields are currently `pub` in many of these structs (event flow predates this lint), so the refactor work in #191 is genuine, not just an audit. |
| Failure mode | A public-profile render reads `event.github_node_id` directly; the rendered packet leaks the GitHub-internal `node_id` that should have been aliased. |

### 5. Cache internals

| Slot | Value |
| --- | --- |
| Invariant | The SQLite-backed `shiplog::cache` module exposes a query API; the raw `rusqlite::Connection`, the schema-version row, and the cache-key construction layout must be private. Cache key construction goes through `CacheKey::new(...)` which centralises the cache-namespace + version + hash algorithm. |
| Boundary | `shiplog::cache` module. Public surface: `ApiCache::get`, `ApiCache::set`, `ApiCache::stats`, etc. Private: `Connection`, `schema_version`, `CacheKey::raw`. |
| Today's surface | `shiplog::cache::*` — already mostly private; the lint protects the boundary from regressing as new accessors are added. |
| Failure mode | A new ingest adapter borrows `cache.connection` directly to run a custom SQL query; the cache schema migrates in a future release; the custom query breaks silently. |

### 6. Policy ledger metadata

| Slot | Value |
| --- | --- |
| Invariant | Every `policy/*.toml` ledger carries a common header (`schema_version`, `policy`, `owner`, `status`). The header lives in a shared `Ledger<Body>` wrapper in `xtask::policy`; downstream consumers (the `cargo xtask check-*` checkers) access the body via the wrapper's accessor, never by destructuring the raw `toml::Table`. |
| Boundary | `xtask::policy` module (in the `xtask` crate). The wrapper struct owns the header validation; checkers read `ledger.body()` and never reach for `ledger.raw_table`. |
| Today's surface | `xtask::policy::*`. The wrapper exists implicitly today (each checker re-parses the header); a future refactor consolidates it. |
| Failure mode | A new checker forgets to validate the common header and silently accepts an unversioned ledger. Drift in the header schema becomes invisible because individual checkers don't enforce it consistently. |

## Activation ladder

`disallowed_fields` is **planned**, never activated for cache-internals — the lane discovered through two probes that `disallowed_fields` is a tier-2 tool, only useful when the protected field genuinely lives on a `pub` type. The sequence:

1. **PR #188**: wrote this document; cross-linked from `policy/clippy-lints.toml` (the existing `[[planned]]` entry's `reason` field). No lint config, no Rust code change.
2. **PR #190**: scaffolded `policy/clippy-protected-fields.toml` with six `[[class]]` entries (one per class above). `cargo xtask check-policy-schemas` accepts the new ledger; lint remains `[[planned]]`.
3. **PR #192**: cache-internals external-call-site audit. Verified by workspace grep that no caller outside `crates/shiplog-cache/src/sqlite.rs` reaches into `ApiCache::{conn, default_ttl, max_size_bytes}`; advanced ledger entry `cpf-0005` to `"accessor-ready"`; added a rustdoc citation on `ApiCache`. *Limitation discovered after merge by the first activation probe*: `clippy::disallowed_fields` is path-scope-blind — it fires on every `self.conn` access, including inside the owning `impl`. So an activation that named `ApiCache::conn` would not be a one-line change against the existing code; it would require either per-method `#[expect]` annotations or an inner-struct split.
4. **PR #194**: inner-struct refactor for cache-internals. Moved `{conn, default_ttl, max_size_bytes}` onto a private `ApiCacheInner` struct that `ApiCache` wraps; threaded every internal access through `self.inner.*`; preserved the public `ApiCache` API exactly. Ledger entry `cpf-0005` advanced to `"clippy-ready"`.
5. **This PR (second activation probe + doctrine pivot)**: probed whether `disallowed_fields` accepts the now-nonexistent `ApiCache::*` paths after the #194 refactor. Result: Clippy emits config-level `"does not refer to a reachable field"` warnings for those paths — not silent acceptance, not a hard error. Those warnings fail under `-D warnings` and cannot be suppressed by `#[expect]` / `#[allow]` (they are not tied to a numbered lint). The activation cannot ship as-specified; targeting the surviving private `ApiCacheInner::*` paths is pointless because the type is already unreachable to external code. So the doctrine pivots: `disallowed_fields` is a *tier-2 tool*, only applicable to classes that keep protected fields on a `pub` type. `cpf-0005` advances to `"type-enforced"`; the type system in #194 is the protection. `policy/clippy-lints.toml`'s `[[planned]] disallowed_fields` reason is rewritten to reflect the per-class evaluation.
6. **Optional follow-up — ledger-driven xtask checker**: a future PR may add `cargo xtask check-protected-fields`, a generic checker that scans the public API surface of each class's `boundary_crate` for regressions — for example, rejecting a new `pub fn raw_connection(&self) -> &Connection` on `ApiCache` or `pub` items added to `ApiCacheInner`. The checker would be ledger-driven (one row per class, the `field_candidates` list provides the protected names). This is the recommended next protection layer for `"type-enforced"` classes; it is **not** a blocker for advancing any other class on the ladder.
7. **Subsequent PRs (per class)**: for each of the remaining five classes (bundle-paths, policy-ledger-metadata, redaction-internals, trust-receipts, source-opaque-ids), repeat the same evaluation shape:
   - external-call-site audit;
   - decide whether the class can take the inner-struct route (preferred) or must keep protected fields on a `pub` type;
   - inner-struct refactor where applicable → advance to `"type-enforced"`;
   - `disallowed_fields` activation where the inner-struct route is not practical → advance to `"lint-active"`.

   **Status as of #216:** complete. All five classes completed step 7 across #209 → #216; the per-class outcomes are summarised in [Outcome (as of #216)](#outcome-as-of-216). No class needed `"lint-active"` — each one either took the inner-struct / `pub(crate)` route or had its protection satisfied procedurally (canonical computer, dedicated gate). The next protection layer is the optional ledger-driven `xtask` checker described in step 6, deferred until a real regression justifies it.

The constraint at every step: **never activate the lint without a working accessor surface**. A direct activation against the current surface would produce 100+ findings on day one, drown the operator in exceptions, and either ship broken or grind to a halt. The ladder makes the protection real one class at a time.

## Per-class audit progress

| Class | Status | Notes |
|---|---|---|
| `cpf-0001` redaction-internals | `type-enforced` | Audit landed post-#208: `DeterministicAliasStore` is `pub(crate)`, so external code cannot name the type; the two protected fields (`key: Vec<u8>` HMAC material and `cache: Mutex<BTreeMap<…>>` alias cache) are implicitly private. The public facade `DeterministicRedactor` holds the alias store as a private field and exposes only accessors. The seam was designed with the right shape from day one — no inner-struct refactor needed. Matches the post-#196 posture of cache-internals via a different path (structural privacy from the start, not retrofit). See `cpf-0001` audit-history comment in `policy/clippy-protected-fields.toml`. |
| `cpf-0002` bundle-paths | `scaffolded` (audited, partially tightened) | Audit landed post-#204 in #206: `RunArtifactPaths::out_dir` was de facto private (no external caller reads it). Visibility-tightening follow-up landed in #207 (`pub` → `pub(crate)`). Bundle/share manifest entry types do not yet exist in the codebase — when they land, apply the inner-struct pattern from day one rather than retrofitting it. `disallowed_fields` activation unlikely to be the right tool per the cache-internals tier-2 framing. See `cpf-0002` audit-history comment in `policy/clippy-protected-fields.toml`. |
| `cpf-0003` trust-receipts | `type-enforced` | Audit landed post-#212: `FileChecksum` is the one existing receipt type; its fields are `pub` by serde-shape necessity (the type IS the bundle-manifest JSON schema). Protection is procedural — single canonical computer in `shiplog::bundle`, determinism property tests, fuzz harness. `disallowed_fields` is the wrong tool (fields must stay `pub` for serde). Other receipt types named in the doc (intake report receipt, cache receipt) don't yet exist; when they land, apply the canonical-computer pattern from day one. See `cpf-0003` audit-history comment in `policy/clippy-protected-fields.toml`. |
| `cpf-0004` source-opaque-ids | `type-enforced` | Audit landed post-#214: the doc's framing turned out to be outdated. Per-source raw `node_id` fields do not exist on adapter event types -- every ingest adapter (`shiplog::ingest::{github,gitlab,jira,linear,git,json,manual}`) populates the single canonical wrapper field `SourceRef::opaque_id: Option<String>` in `shiplog::schema`. The field is `pub` by serde-shape necessity (the event ledger JSON contract). Protection is procedural: `shiplog::redact::policy::redact_event_with_aliases` sets `event.source.opaque_id = None` for non-internal profiles; property + integration tests prove the nullification works. `disallowed_fields` is the wrong tool. See `cpf-0004` audit-history comment in `policy/clippy-protected-fields.toml`. |
| `cpf-0005` cache-internals | `type-enforced` | Audit + refactor + probe complete: #192 → #194 → #196. Type system is the protection mechanism; lint not activated. |
| `cpf-0006` policy-ledger-metadata | `type-enforced` | Audit landed post-#210: the wrapper type `LoadedPolicy` does exist in `xtask::policy`. Two structural pillars protect the seam: (a) `xtask` is `publish = false`, so external code cannot depend on the crate or reach the type; (b) `cargo xtask check-policy-schemas` is the dedicated header-validation gate that runs in `blocking-allowlist` mode on every PR. `LoadedPolicy::{path, header, raw}` are currently `pub` within `xtask` — a tiny `pub(crate)` tightening (mirroring #208's `RunArtifactPaths::out_dir` change) is framed as a candidate follow-up. `disallowed_fields` is the wrong tool for this class. See `cpf-0006` audit-history comment in `policy/clippy-protected-fields.toml`. |

## Out of scope

- **Test-side carveouts.** Per the operating contract, tests do not get blanket Clippy carveouts. Where a test legitimately reaches into a protected field (e.g. asserting an alias map state), use `#[expect(clippy::disallowed_fields, reason = "policy:test/...")]` with a policy ID, not a `#[allow(...)]`.
- **`pub(crate)` vs accessor.** Some seams may end up as `pub(crate)` rather than full accessors — that's fine for boundaries that don't need a normalising step, as long as the lint config and `clippy-protected-fields.toml` match what was actually chosen.
- **External types.** Foreign struct fields (from `serde_json::Value`, `rusqlite::Row`, etc.) are not protected by this lint. The boundary is at the shiplog crate edge.

## See also

- [`policy/clippy-protected-fields.toml`](../policy/clippy-protected-fields.toml) — the scaffolded ledger that encodes the six classes above per-entry (id, slug, invariant, boundary, accessor pattern, field candidates, activation status).
- [`policy/clippy-lints.toml`](../policy/clippy-lints.toml) — the current Clippy lint ledger, where `clippy::disallowed_fields` lives as a `[[planned]]` entry until the protected-fields scaffold lands.
- [`docs/CLIPPY_POLICY.md`](CLIPPY_POLICY.md) — overall Clippy doctrine, lint ratchets, suppression style.
- [`docs/POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) — the common header schema every policy ledger shares (the "policy ledger metadata" seam above).
