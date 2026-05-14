# Crate-Surface Contraction Implementation Plan

Status: active
Owner: product/release
Created: 2026-05-14
Target release: 0.7.0

Related proposal:
[`SHIPLOG-PROP-0003-crate-surface-contraction`](../../docs/proposals/SHIPLOG-PROP-0003-crate-surface-contraction.md)
Related spec:
[`SHIPLOG-SPEC-0004-public-crate-support-tiers`](../../docs/specs/SHIPLOG-SPEC-0004-public-crate-support-tiers.md)
Related ADRs:
[`SHIPLOG-ADR-0004-srp-modules-over-public-microcrates`](../../docs/adr/SHIPLOG-ADR-0004-srp-modules-over-public-microcrates.md),
[`SHIPLOG-ADR-0005-json-schemas-over-public-rust-schema-crate`](../../docs/adr/SHIPLOG-ADR-0005-json-schemas-over-public-rust-schema-crate.md)
Related audit:
[`docs/release/0.7-crate-surface.md`](../../docs/release/0.7-crate-surface.md)
Related policy:
[`policy/publish-allowlist.toml`](../../policy/publish-allowlist.toml)

## Objective

Prepare shiplog 0.7.0 as the crate-surface contraction release. The release
must publish only the supported `shiplog` package and must not rely on 0.6.x
implementation crates as hidden public dependencies.

The product behavior from 0.6.0 remains the guardrail:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

No Evidence Repair Loop behavior belongs in this lane.

## Current Receipts

Landed:

- [#250](https://github.com/EffortlessMetrics/shiplog/pull/250):
  `docs(proposal): add 0.7 crate-surface contraction proposal`
- [#251](https://github.com/EffortlessMetrics/shiplog/pull/251):
  `docs(spec): define public crate support tiers`
- [#252](https://github.com/EffortlessMetrics/shiplog/pull/252):
  `docs(adr): prefer SRP modules over public microcrates`
- [#253](https://github.com/EffortlessMetrics/shiplog/pull/253):
  `docs(crate-surface): classify workspace crates`
- [#254](https://github.com/EffortlessMetrics/shiplog/pull/254):
  `refactor(crate): inline render crates`
- [#255](https://github.com/EffortlessMetrics/shiplog/pull/255):
  `chore(release): enforce 0.7 publish allowlist`
- [#256](https://github.com/EffortlessMetrics/shiplog/pull/256):
  `chore(release): mark non-public crates unpublished`
- [#257](https://github.com/EffortlessMetrics/shiplog/pull/257):
  `docs(release): document 0.6 crate surface as historical`
- [#258](https://github.com/EffortlessMetrics/shiplog/pull/258):
  `docs(adr): make json schemas the public contract`
- [#259](https://github.com/EffortlessMetrics/shiplog/pull/259):
  `plans(crate-surface): add implementation plan and active goal`
- [#260](https://github.com/EffortlessMetrics/shiplog/pull/260):
  `refactor(crate): inline local git adapter`
- [#261](https://github.com/EffortlessMetrics/shiplog/pull/261):
  `refactor(crate): inline linear adapter`

The remaining risk is dependency leakage: `shiplog` can be publish-allowed but
still resolve historical 0.6 implementation crates if versioned path
dependencies are left in its manifest. 0.7.0 should remove that dependency graph
rather than hide it behind the publish allowlist.

## Sequencing

### PR 1: Collapse Ingest Adapters

Title:

```text
refactor(crate): inline ingest adapters into shiplog
```

Scope:

- Move source adapter code from `crates/shiplog-ingest-*` into
  `apps/shiplog/src/ingest/` owner modules in reviewable slices.
- Continue from the landed `shiplog-ingest-git` slice, then collapse remaining
  source adapters in reviewable order.
- Migrate adapter tests or keep equivalent product tests in the new package
  location.
- Preserve recorded GitHub fixture proof.
- Remove direct `shiplog` dependencies on `shiplog-ingest-*` packages.

Expected files:

- `apps/shiplog/Cargo.toml`
- `apps/shiplog/src/ingest/**`
- `apps/shiplog/src/main.rs`
- `apps/shiplog/tests/**`
- `crates/shiplog-ingest-*/**`
- `Cargo.toml`
- `docs/release/0.7-crate-surface.md`
- `policy/publish-allowlist.toml` only if package inventory changes

Behavior change:

- None intended.
- Source identity, skipped freshness, stale freshness, and next-step output must
  remain unchanged.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo test -p shiplog-ingest-github recorded_http_fixtures_prove_full_fresh_then_cached_ingest
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

When a package has been removed, use the equivalent moved test path and record
that mapping in the PR body.

Rollback:

- Restore the adapter crates and their `shiplog` manifest dependencies.
- Revert import rewrites and moved tests together.

Follow-up:

- Collapse cache/freshness and trust support that adapters still depend on.

### PR 2: Collapse Cache And Freshness Support

Title:

```text
refactor(crate): inline cache freshness support
```

Scope:

- Move `shiplog-cache` into `apps/shiplog/src/cache/` or the closest owner
  module.
- Preserve `CacheLookup::{Fresh, Stale, Miss}` semantics and stale proof.
- Remove `shiplog` dependency on `shiplog-cache`.

Expected files:

- `apps/shiplog/Cargo.toml`
- `apps/shiplog/src/cache/**`
- `apps/shiplog/tests/**`
- `crates/shiplog-cache/**`
- `docs/release/0.7-crate-surface.md`

Behavior change:

- None intended.
- Fresh/cached/stale/skipped/unavailable semantics must remain receipt-backed.

Validation:

```bash
cargo test -p shiplog-cache
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test intake_cold_start
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Restore `shiplog-cache` and adapter imports.

Follow-up:

- Tighten publish-dependency checks after the cache dependency is gone.

### PR 3: Collapse Product And Trust Support

Title:

```text
refactor(crate): inline product trust support
```

Scope:

- Move `shiplog-bundle`, `shiplog-redact`, `shiplog-coverage`,
  `shiplog-workstreams`, `shiplog-merge`, and `shiplog-team` in small slices.
- Keep bundle, redaction, coverage, and workstream output shape unchanged.
- Preserve fail-closed redaction behavior.

Expected files:

- `apps/shiplog/Cargo.toml`
- `apps/shiplog/src/{bundle,redact,coverage,workstreams,merge,team}/**`
- `apps/shiplog/tests/**`
- `crates/shiplog-{bundle,redact,coverage,workstreams,merge,team}/**`
- `docs/release/0.7-crate-surface.md`

Behavior change:

- None intended.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Restore the collapsed crate and package imports for the failed slice only.

Follow-up:

- Move foundation contracts after higher-level callers are in one package.

### PR 4: Collapse Foundation Contracts

Title:

```text
refactor(crate): inline internal foundation contracts
```

Scope:

- Move `shiplog-ids`, `shiplog-schema`, and `shiplog-ports` into internal
  owner modules.
- Preserve JSON schema files under `contracts/schemas/` as the public machine
  contract.
- Remove `shiplog` dependencies on the foundation crates.

Expected files:

- `apps/shiplog/Cargo.toml`
- `apps/shiplog/src/{ids,schema,ports}/**`
- `contracts/schemas/**` only if generated schema paths require updates
- `crates/shiplog-{ids,schema,ports}/**`
- `docs/release/0.7-crate-surface.md`

Behavior change:

- None intended.
- Report JSON shape and schema validation must remain compatible unless a
  separate schema PR says otherwise.

Validation:

```bash
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test cli_integration -- report
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Restore foundation crates and imports together.

Follow-up:

- Enforce that publish-allowed packages do not depend on non-public workspace
  packages or historical implementation crates.

### PR 5: Enforce Publish Dependency Closure

Title:

```text
chore(release): enforce 0.7 package dependency closure
```

Scope:

- Extend package-boundary or release proof so a publish-allowed package cannot
  depend on non-public workspace packages as registry dependencies.
- Prove `shiplog` is the only package in the release matrix.
- Prove `cargo publish --dry-run -p shiplog --locked` does not rely on 0.6
  implementation crates.

Expected files:

- `scripts/package-boundary-audit.sh`
- `scripts/package-proof.sh`
- `scripts/publish-dry-run.sh`
- `policy/publish-allowlist.toml`
- `docs/release/0.7-crate-surface.md`

Behavior change:

- None.

Validation:

```bash
bash scripts/package-boundary-audit.sh
bash scripts/package-proof.sh
bash scripts/publish-dry-run.sh
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert the release-proof tightening only if it blocks a valid package graph
  and record the reason in the PR.

Follow-up:

- Prepare the 0.7 release once dependency closure is enforced.

### PR 6: Release Prep

Title:

```text
release: prepare 0.7.0 crate-surface release
```

Scope:

- Prepare version, changelog/release notes, package proof, and release handoff.
- Record compatibility posture for historical 0.6 implementation crates.
- Keep Evidence Repair Loop out of the release.

Expected files:

- `CHANGELOG.md`
- `docs/release/**`
- release handoff files
- package scripts only if release proof finds drift

Behavior change:

- Package-surface behavior only.
- User-facing first-run behavior should remain unchanged.

Validation:

```bash
cargo xtask package-version
cargo xtask package-boundary
cargo test -p shiplog --test intake_cold_start
cargo test -p shiplog --test front_door_first_pack_smoke
cargo test -p shiplog --test cli_integration -- intake
cargo test -p shiplog --test cli_integration -- report
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo xtask check-policy-schemas
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-executable-files --mode blocking-allowlist
git diff --check
```

Rollback:

- Revert release-prep docs and version changes; do not roll back landed
  contraction refactors unless they caused the release blocker.

Follow-up:

- Archive `.shiplog/goals/active.toml` after 0.7 ships.
- Start Evidence Repair Loop only after the 0.7 release closure receipt lands.

## Stop Conditions

Stop and split the PR if:

- report JSON shape changes;
- Markdown packet output shape changes;
- source freshness semantics change;
- a moved test requires live network;
- a Cargo feature is removed or renamed;
- a package collapse needs a public Rust API decision;
- Evidence Repair Loop behavior appears in the diff.

Stop and write an ADR if:

- a crate appears to have a real external Rust consumer;
- an adapter/plugin API becomes a release requirement;
- `shiplog-schema` needs to become public again;
- the release must intentionally depend on a historical 0.6 implementation
  crate.

## Lane Closure

Close the lane only when:

- `shiplog` is the only 0.7 publish-eligible package;
- `cargo publish --dry-run -p shiplog --locked` succeeds;
- the published package graph does not require unsupported implementation
  crates as forward dependencies;
- first-run intake/open/report behavior has targeted proof;
- 0.7 release prep records install, package, and compatibility proof;
- the active goal is archived with shipped release receipts.
