# Policy Ledgers

Orientation to the [`policy/`](../../policy/) directory: machine-readable
TOML ledgers that encode shiplog's CI economics, lint policy, no-panic
baseline, and file/process/network policy.

This is the docs-side companion to
[`policy/README.md`](../../policy/README.md). The README has the
canonical inventory; this doc explains the architecture and how the
ledgers are consumed.

## Architecture

```text
policy/                      ← machine-readable ledgers (TOML)
docs/ci/                     ← human-readable operating contracts (Markdown)
contracts/schemas/           ← JSON Schemas for emitted artifacts
xtask/src/tasks/             ← Rust-native runners that load and validate (PR #143+)
.github/workflows/           ← GitHub Actions that consume the runners (PR #146+)
```

The flow:

```text
policy/*.toml  ──→  xtask checker  ──→  step summary + receipt artifact
                                                ↓
docs/ci/*.md  ←─  human reference ←─  policy ledger semantics
                                                ↓
contracts/schemas/*.json  ──→  validates the receipt artifact
```

Each ledger is **declared once** (in `policy/`), **explained in prose**
(in `docs/ci/`), and **consumed by checkers** (in `xtask/`). When the
explanations and the TOML disagree, the TOML wins; the docs are kept
in sync as a separate review concern.

## Why TOML

TOML is the right format for these ledgers because:

- Hand-editable; reviewers can read the diff without tooling.
- Comment support; entries carry the `reason` inline plus header
  comments.
- Strict syntax; TOML parsers reject ambiguity (no YAML-style
  whitespace surprises).
- Native to Rust tooling (Cargo.toml, rust-toolchain.toml, deny.toml,
  clippy.toml).
- The Python `tomllib` (3.11+) makes it trivial to validate without
  a Rust toolchain installed.

## The 18 ledgers

Grouped by concern:

| Concern | Ledgers | Owner | Consumed by |
|---|---|---|---|
| **CI economics** | ci-budget, ci-lanes, ci-risk-packs, ci-exceptions | release/ci | PR plan (#146) |
| **Clippy** | clippy-lints, clippy-debt, clippy-exceptions | policy | Clippy ledger checker (#150) |
| **No-panic** | no-panic-baseline, no-panic-allowlist | policy | no-panic checker (#151) |
| **File policy (master)** | non-rust-allowlist, non-rust-debt | policy | file-policy checker (#149) |
| **File policy (companions)** | generated, executable, workflow, dependency-surface, process, network | policy/release | file-policy companion checkers (#149) |
| **ripr** | ripr-suppressions | policy | ripr lane (#153) |

Plus `policy/README.md` for orientation.

## Common receipt fields

Every ledger has a four-line header:

```toml
schema_version = 1
policy = "<ledger-name>"
owner = "EffortlessMetrics"
status = "advisory"
```

`status` will move from `"advisory"` to `"blocking"` per ledger as the
checker that loads it lands and the team accepts the enforcement. In
v0.5.0, every ledger stays advisory.

Every entry has the four reviewer questions (per
[`POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md)):

```toml
id = "<stable-identifier>"
owner = "<workspace-package or release|policy|ci|docs>"
reason = "<one-or-two-sentence explanation>"
created = "<ISO date>"
review_after = "<ISO date>"
expires = "<ISO date or 'permanent'>"
```

Plus per-ledger extension fields (`family` + `selector_kind` for
no-panic; `permissions` + `external_actions` for workflow; `binary` +
`argv_shape` + `network_reach` for process; etc.). Each ledger's
header comment documents its extension fields.

## Per-ledger docs

| Ledger | Docs |
|---|---|
| ci-budget | [`lem-budgeting.md`](lem-budgeting.md), [`cost-and-verification-policy.md`](cost-and-verification-policy.md) |
| ci-lanes | [`ci-lane-map.md`](ci-lane-map.md), [`test-evidence-lanes.md`](test-evidence-lanes.md) |
| ci-risk-packs | [`risk-packs.md`](risk-packs.md) |
| ci-exceptions | [`cost-and-verification-policy.md`](cost-and-verification-policy.md) (Rule 3) |
| clippy-lints, clippy-debt, clippy-exceptions | [`../CLIPPY_POLICY.md`](../CLIPPY_POLICY.md) |
| no-panic-baseline, no-panic-allowlist | [`../NO_PANIC_POLICY.md`](../NO_PANIC_POLICY.md) |
| non-rust-allowlist, non-rust-debt | [`../FILE_POLICY.md`](../FILE_POLICY.md) |
| generated, executable, workflow | [`../FILE_POLICY.md`](../FILE_POLICY.md) |
| dependency-surface | [`../FILE_POLICY.md`](../FILE_POLICY.md) |
| process, network | [`../FILE_POLICY.md`](../FILE_POLICY.md) |
| ripr-suppressions | [`ripr.md`](ripr.md) |

The common allowlist schema and suppression style is documented once in
[`../POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md).

## Lifecycle

A ledger's lifecycle:

1. **Skeleton.** TOML file added with header + entry-shape comment.
   Status: `"advisory"`. (PR #141 for all 18.)
2. **Operating contract.** Markdown doc added explaining what the
   ledger means and how it's consumed. (PR #142.)
3. **Checker.** xtask command added that validates the ledger and
   reports drift. (PR #143 for the runner, #149/#150/#151/#153 for
   per-ledger checkers.)
4. **Workflow consumer.** GitHub Actions workflow added that runs the
   checker on PR. (PR #146 for ci-* ledgers, #149/#150/#151/#153 for
   the rest.)
5. **Promotion.** Status moves from `"advisory"` to `"blocking"` once
   the checker has been observed stable on `main` and the team
   accepts the enforcement. Each promotion is its own PR with
   rationale.

In v0.5.0, the rollout reaches step 4 for all ledgers. Step 5
(promotion to blocking) is deferred to a follow-up release.

## Adding a new ledger

A new ledger:

1. Lands first in `policy/` with the standard header + entry-shape
   comment + initial seed entries (if obvious).
2. Adds an entry to `policy/README.md`'s file table.
3. Adds an explanatory doc under `docs/ci/` or `docs/`.
4. Updates this overview's grouping table.
5. (When the checker exists) Adds a `cargo xtask check-<name>`
   command and wires it into the policy report.

A removed ledger:

1. Removes the TOML file.
2. Removes the README entry, the docs, and any references.
3. Removes the xtask checker.
4. Removes the workflow consumer.
5. Notes the removal in the PR description so reviewers see the policy
   shrink.

## See also

- [`policy/README.md`](../../policy/README.md) — file inventory + receipt schema
- [`../POLICY_ALLOWLISTS.md`](../POLICY_ALLOWLISTS.md) — common allowlist schema
- [`../CLIPPY_POLICY.md`](../CLIPPY_POLICY.md), [`../NO_PANIC_POLICY.md`](../NO_PANIC_POLICY.md), [`../FILE_POLICY.md`](../FILE_POLICY.md) — per-policy doctrine
- [`rust-1.95-rollout.md`](rust-1.95-rollout.md) — how the ledgers fit the 18-PR ladder
