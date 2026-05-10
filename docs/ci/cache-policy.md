# Cache Policy

How shiplog uses GitHub Actions caches and the `Swatinem/rust-cache`
action across CI lanes. Designed to keep the per-PR LEM cost down
without losing build speed on hot paths.

This pairs with [`lem-budgeting.md`](lem-budgeting.md) (the cost model)
and [`cost-and-verification-policy.md`](cost-and-verification-policy.md)
(the doctrine). Implementation lands in PR #147.

## The four rules

1. **PRs restore caches; PRs do not save.** A PR job restores from a
   cache produced by `main` and reports a cache hit/miss. It does not
   write a new cache key. This avoids cache fan-out across hundreds of
   PR-shaped key combinations.
2. **`main` saves canonical caches.** Push-to-`main` jobs save under
   the canonical cache key for that lane. The `main` build is the
   producer; PRs are consumers.
3. **Matrix shards mostly restore-only.** Each matrix leaf restores
   the same key as its `main` counterpart. No matrix leaf saves a
   shard-specific cache.
4. **Release uses release-specific cache keys.** The release pipeline
   has its own cache keys (`release-preflight`, per-target build
   caches) so it does not pollute the PR-fast cache.

## Why this shape

Without rule 1, every PR writes a fresh cache key, GitHub Actions
caches expire on a 7-day LRU, and the next push-to-`main` finds the
cache cold. The result: every PR is fast (~12 LEM check) but every
push-to-`main` is slow (~25 LEM check), which is the wrong trade-off.

With rule 1, PRs read the warm `main` cache and report a hit. PRs that
miss are slow (no fix), but the `main` cache stays the warm anchor.

Without rule 4, a release pipeline's heavy multi-target build pollutes
the PR-fast `main` cache key, and the next PR waits ~5 LEM extra to
restore an oversized cache that includes Linux+macOS+Windows binary
artifacts the PR doesn't need.

## Cache key strategy

### `Swatinem/rust-cache` shared keys

| Lane | `shared-key` | Saves on `main`? | Restores on PR? |
|---|---|---|---|
| `lane.ci_check` Ubuntu | `check-ubuntu-latest` | yes | yes |
| `lane.ci_check_windows` | `check-windows-latest` | yes | yes |
| `lane.ci_deny` | `deny` | yes | yes |
| `lane.ci_msrv` | `msrv` | yes | yes |
| `lane.coverage` | `coverage-1.95` (renamed from `coverage-1.92` in PR #145) | yes (only on push main per coverage.yml current logic) | yes (label-gated) |
| `lane.bdd` (4 jobs) | `bdd-<job>` | yes | yes |
| `lane.property` | `property` | yes | yes |
| `lane.fuzz_*` | `fuzz` (separate from main workspace) | yes | yes |
| `lane.mutation_*` | `mutation` | yes | n/a (off PR) |
| `lane.release_*` | `release-preflight`, `release-build-<target>` | n/a (tag-only) | n/a |

### `actions/cache` direct keys

The fuzz workflow uses `actions/cache` for `~/.cargo/bin` because
`cargo-fuzz` is shared across fuzz target invocations. Same restore-only
rule applies: PR restores, `main` (or scheduled) saves.

## How rule 1 is implemented

`Swatinem/rust-cache@v2` exposes a `save-if` parameter:

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "check-ubuntu-latest"
    save-if: ${{ github.ref == 'refs/heads/main' }}
```

The `coverage.yml` workflow already does this (`save-if: ${{ github.ref == 'refs/heads/main' }}`).

PR #147 applied the pattern to the rest (now landed):

- `ci.yml` (`check`, `deny`, `msrv`) — `save-if: ${{ github.ref == 'refs/heads/main' }}` on each
- `bdd-testing.yml` (5 jobs) — distinct `shared-key` per job + main-only save
- `property-testing.yml` — `shared-key: property` + main-only save
- `fuzzing.yml` (quick + extended) — `shared-key: fuzz` + save on main or scheduled cron
- `mutation-testing.yml` — `shared-key: mutation` + save on main or `workflow_dispatch`

## Docs-only PRs skip compile

A PR that matches only the `docs-only` risk pack should not pay any
compile cost. PR #147 added path filters to the compile-heavy
non-required workflows:

```yaml
on:
  pull_request:
    paths-ignore:
      - "docs/**"
      - "*.md"
      - "**/*.md"
```

Applied to `bdd-testing.yml`, `property-testing.yml`, and
`fuzzing.yml` (quick lane). Not applied to `ci.yml` because its `check`
/ `deny` / `msrv` jobs are required-merge gates and a missing required
check would deadlock docs-only PRs (see
[`branch-protection.md`](branch-protection.md)). On a docs-only PR
post-#147:

- `ci.yml` runs (required) — fast on docs because no Rust changes; cache
  hits `main`'s warm cache
- `pr-plan / forecast` runs (1 LEM)
- `coverage.yml` already path-gated to Rust surfaces — naturally skips
- `bdd-testing.yml`, `property-testing.yml`, `fuzzing.yml` quick — all
  skip via `paths-ignore`
- `mutation-testing.yml`, extended fuzz — skip via non-PR triggers
- bots (CodeRabbit / Gemini / Droid — advisory) run unchanged
- GitGuardian (push-event) runs unchanged

The PR plan reports the path-filtered skips as `skipped: docs-only` (see
[`skipped-by-policy.md`](skipped-by-policy.md)).

## What rule 4 means in practice

Today (post-#142) the release pipeline uses these cache keys:

- `actions/cache@v5` with key `${{ runner.os }}-cargo-registry-${{ hashFiles('**/Cargo.lock') }}` (cargo registry)
- `actions/cache@v5` with key `${{ runner.os }}-cargo-index-${{ hashFiles('**/Cargo.lock') }}` (cargo index)
- `actions/cache@v5` with key `${{ runner.os }}-${{ matrix.target }}-cargo-release-${{ hashFiles('**/Cargo.lock') }}` (per-target build)

These are runner-OS-scoped and per-target, so they don't collide with
`Swatinem/rust-cache@v2` keys used by PR-fast lanes. Rule 4 is already
satisfied; PR #147 documents this and adds the matching `save-if` /
`restore-only` semantics where they're missing.

## Validation

PR #147's acceptance criteria (now landed):

- Every PR-time job uses `save-if: ${{ github.ref == 'refs/heads/main' }}`
  on `Swatinem/rust-cache@v2` (or, for `mutation-testing.yml` and
  `fuzzing.yml` extended, the equivalent `main || schedule` /
  `main || workflow_dispatch` form so the producer is the long-running
  variant of each lane).
- Every push-to-`main` run produces or refreshes the canonical cache.
- A docs-only PR's compile-heavy non-required workflows skip via
  `paths-ignore`; reviewers see the skips called out in the PR plan
  step summary.
- The release pipeline cache keys (`release-preflight` shared-key plus
  per-target `actions/cache` keys) remain segregated from PR-fast keys.

The actuals to confirm the LEM drop land in PR #148.

## See also

- [`policy/ci-budget.toml`](../../policy/ci-budget.toml) — LEM budget context
- [`lem-budgeting.md`](lem-budgeting.md) — cost model
- [`ci-actuals.md`](ci-actuals.md) — how cache hits are recorded
- [`ci-lane-map.md`](ci-lane-map.md) — per-lane cache shared-key
