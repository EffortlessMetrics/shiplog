# Policy Allowlists — Common Schema

shiplog's policy ledgers (Clippy debt, no-panic baseline + allowlist, non-Rust
file allowlists, workflow / process / network ledgers) share a common entry
schema and suppression style. This document is the single place to look up
what a ledger entry means.

It is part of the [Rust 1.95 / 0.5.0 quality
rollout](ci/rust-1.95-rollout.md). The ledger TOML skeletons land in PR 2;
the checkers land in PRs 7 (file policy), 8 (Clippy), and 9 (no-panic).
Everything below describes the intended steady state.

## Why a common schema

Without one, every policy ledger invents its own field names. Reviewers stop
reading. Owners lose track of what they signed up for. Burndown becomes
impossible to plan.

The common schema fixes the four review questions:

```text
Where is the thing?     path
Who owns it?            owner
Why is it there?        reason
When does it expire?    expiry
```

Plus per-ledger fields specific to the policy (e.g. `family` and `count` for
no-panic, `permissions` for workflows).

## Core schema

```toml
schema_version = "1.0"

[[entry]]
path = "crates/shiplog-engine/src/intake.rs"
owner = "shiplog-engine"
reason = "Intake config can lack a report_dir when the user has not run init; we surface this through the doctor flow."
expiry = "2026-09-01"
```

Field rules:

- `path` — repository-relative path (forward slashes). Globs allowed only
  where the policy explicitly says so (e.g. `tests/**/*.json`); single-file
  paths are preferred for blast-radius reasons.
- `owner` — the crate or sub-team that takes responsibility. Use the
  workspace package name (`shiplog-engine`, `shiplog-redact`, etc.) when the
  finding lives in that package, or one of: `release`, `policy`, `ci`,
  `docs`.
- `reason` — one or two sentences. Explain the situation, not the
  workaround. A future maintainer should be able to decide whether the entry
  is still load-bearing without reading commit history.
- `expiry` — ISO-8601 date when the entry should be revisited, or the literal
  string `"permanent"` for entries that are not expected to age out (e.g. a
  workflow file that will always exist).

Per-ledger schemas extend this base — see the relevant policy doc.

## Suppression style

Where the policy needs an in-source suppression (Clippy, in particular), we
use `#[expect(..., reason = "...")]` rather than `#[allow(..., ...)]`:

```rust
#[expect(
    clippy::needless_pass_by_value,
    reason = "kept until shiplog-engine cleanup; tracked in policy/clippy-debt.toml",
)]
fn render_packet(events: Vec<Event>) -> Packet { /* ... */ }
```

`expect` fails the build when the lint stops firing, which kills dead
suppressions automatically. Bare `#[allow(...)]` is rejected by the policy
checker — it is the thing the policy is designed to prevent.

For suppressions that need a longer justification than a `reason` string,
cite the policy entry:

```rust
#[expect(
    clippy::cognitive_complexity,
    reason = "see policy/clippy-exceptions.toml#shiplog-engine.collect_multi",
)]
fn collect_multi(/* ... */) -> Result<Run> { /* ... */ }
```

The cited entry in `policy/clippy-exceptions.toml` then carries the full
explanation.

## Expiry handling

`expiry` dates are advisory in the sense that nothing breaks on the day they
pass — but `cargo xtask policy-report` highlights expired entries, and a
release PR (PRs 14–15 in the rollout) is expected to either renew, fix, or
remove every expired entry before tag.

`permanent` means: do not expect this to age out. Use sparingly. A workflow
file is a fair `permanent`; a `needless_pass_by_value` debt entry is not.

## Ownership

`owner` is who the policy reviewer pings when the entry needs attention.
Owners may be:

- a workspace package name (the package containing the finding) — most
  common,
- `release` (release scripts, version alignment, package boundary),
- `policy` (the rollout itself, ledger schema, xtask checkers),
- `ci` (workflow files, lane economics, runner pinning),
- `docs` (README, docs/**/*.md, schema docs).

If an entry's owner cannot be named, the entry should not be added.

## Per-ledger extension fields

| Ledger | Extra fields |
| ------ | ------------ |
| `policy/clippy-lints.toml` | `name` (lint), `level` (warn/deny/forbid), `class`, `activate_when_msrv` (for `[[planned]]`) |
| `policy/clippy-debt.toml` | `lint`, `lint_level`, `crate_scope` |
| `policy/clippy-exceptions.toml` | `lint`, `module_path`, `cited_from` |
| `policy/no-panic-baseline.toml` | `family`, `selector_kind`, `selector_callee`, `snippet`, `count` (no `expiry` — baseline entries are not approvals) |
| `policy/no-panic-allowlist.toml` | `family`, `selector_kind`, `selector_callee`, `snippet` |
| `policy/non-rust-allowlist.toml` | `category` (script / fixture / config / docs / fuzz / generated) |
| `policy/generated-allowlist.toml` | `generator` (tool that produces it), `regenerate_command` |
| `policy/executable-allowlist.toml` | `interpreter` (e.g. `bash`, `pwsh`) |
| `policy/dependency-surface-allowlist.toml` | `surface` (Cargo.toml / workflow / lockfile), `dep_count_at_baseline` |
| `policy/workflow-allowlist.toml` | `permissions[]`, `secrets_used[]`, `external_actions[]` |
| `policy/process-allowlist.toml` | `binary`, `argv_shape`, `network_reach` (bool) |
| `policy/network-allowlist.toml` | `destination`, `auth_required` (bool), `lane` (build / test / release) |

Refer to each policy document for the rationale behind its extension fields.

## Validation

The `xtask` runner (PR 3 onward) provides:

```bash
cargo xtask check-lint-policy            # Clippy
cargo xtask check-clippy-exceptions
cargo xtask check-no-panic-family        # no-panic
cargo xtask check-file-policy            # non-Rust master
cargo xtask check-generated
cargo xtask check-executable-files
cargo xtask check-dependency-surfaces
cargo xtask check-workflow-surfaces
cargo xtask check-process-policy
cargo xtask check-network-policy
cargo xtask policy-report                # combined human summary
```

Each checker reports unknown entries (in the file but not in the source),
unmapped findings (in the source but not in the file), expired entries, and
missing owners.

## See also

- [`CLIPPY_POLICY.md`](CLIPPY_POLICY.md)
- [`NO_PANIC_POLICY.md`](NO_PANIC_POLICY.md)
- [`FILE_POLICY.md`](FILE_POLICY.md)
- [`ci/rust-1.95-rollout.md`](ci/rust-1.95-rollout.md)
