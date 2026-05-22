# xtask

`xtask` is shiplog's Rust-native control plane for policy checks and
release-proof aggregation. It lives under [`xtask/`](../xtask/) as a
workspace member with `publish = false`.

This doc lands in PR #143 (the foundation). The four initial commands
are minimal; per-ledger checkers (file/Clippy/no-panic/ripr) and the
PR-plan/actuals lanes are added in later PRs.

## Invocation

The workspace [`.cargo/config.toml`](../.cargo/config.toml) provides an
alias:

```bash
cargo xtask <subcommand>
# expands to: cargo run --quiet -p xtask -- <subcommand>
```

## Commands

### `cargo xtask check-policy-schemas`

Validates every `policy/*.toml` file for a well-formed common header:

- `schema_version = 1`
- `policy = "<file stem>"` (matches filename)
- `owner = "<owner>"`
- `status = "advisory"` or `"blocking"`

Exits with non-zero if any finding. See
[`docs/POLICY_ALLOWLISTS.md`](POLICY_ALLOWLISTS.md) for the schema.

### `cargo xtask check-doc-artifacts`

Validates [`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml), the
source-of-truth ledger for proposals, specs, ADRs, and plans:

- artifact IDs are unique;
- kind and status values are recognized;
- artifact paths exist and match the declared kind;
- the artifact ID is mentioned in the artifact file;
- linked proposal/spec/ADR/plan IDs resolve to ledger entries; and
- superseded artifacts point at a replacement.

Exits with non-zero if any finding. This is the dedicated proof command for
the document artifact link support-tier surface.

### `cargo xtask check-goals`

Validates [`.codex/goals/active.toml`](../.codex/goals/active.toml), the
Codex-facing execution-state manifest:

- the active goal manifest exists and parses;
- goal and work-item status values are recognized;
- work-item IDs are unique;
- at most one work item is `active`;
- proposal and spec references resolve to artifact IDs in
  [`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml);
- the plan reference is a safe repo-relative path, exists, and is ledgered as a
  plan artifact;
- the work-item ID is listed in the referenced implementation plan;
- ready and active work items carry proof commands;
- blocked work items name a blocker; and
- done work items carry proof commands or receipt refs.

Exits with non-zero if any finding. This is the dedicated proof command for
the active-goal support-tier surface.

### `cargo xtask check-support-tiers`

Validates [`docs/status/SUPPORT_TIERS.md`](status/SUPPORT_TIERS.md), the
source-of-truth map from product/governance claims to proof commands:

- the claim map table exists and has the expected columns;
- support-tier names are recognized;
- stable and stabilizing claims have backticked proof commands; and
- `cargo xtask ...` proof commands reference known xtask subcommands.

Exits with non-zero if any finding. This is the dedicated proof command for
the support-tier claim-map surface.

### `cargo xtask package-boundary`

Verifies published vs dev-only crate classification. Delegates to
`scripts/package-boundary-audit.sh` until Rust parity is proven.
Requires `bash` + `python3`; not supported on Windows in this PR (run
via WSL or Git Bash, or invoke the script directly). Existing CI runs
the script directly on Ubuntu, unchanged.

### `cargo xtask package-version`

Verifies workspace package version alignment. Same delegation /
platform constraints as `package-boundary`.

### `cargo xtask policy-report`

Prints a human summary of every policy ledger: file name, status, and
the count of top-level array-of-table entries. Useful for spot-checking
ledger growth without opening each file.

### `cargo xtask repo-contract-report`

Writes source-of-truth graph reports for humans and agents:

- `target/source-of-truth/graph.json`
- `target/source-of-truth/graph.md`

The report reads [`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml),
[`.codex/goals/active.toml`](../.codex/goals/active.toml), and
[`docs/status/SUPPORT_TIERS.md`](status/SUPPORT_TIERS.md). It summarizes the
active goal, work items, artifact links, support-tier proof commands, and graph
edges. It does not mutate source artifacts and does not replace the dedicated
validators.

### `cargo xtask pr-body`

Generates a draft pull request body from a work item in
[`.codex/goals/active.toml`](../.codex/goals/active.toml):

```bash
cargo xtask pr-body --work-item pr-body-generator
```

The command reads the active goal manifest, the linked implementation plan, and
the ledgered proposal/spec/ADR refs in
[`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml). By default it
writes:

- `target/source-of-truth/pr-body.md`

The generated body includes proposal, spec, ADR, plan item, scope, non-goals,
support-tier impact, policy impact, proof commands, claim boundary, and
rollback when those fields are present in the linked plan/spec. It is a derived
draft only: it does not call the GitHub API, create a PR, mutate source
artifacts, change branch protection, or replace reviewer judgment.

### `cargo xtask closeout`

Generates source-of-truth closeout artifacts from
[`.codex/goals/active.toml`](../.codex/goals/active.toml):

```bash
cargo xtask closeout --goal shiplog-source-of-truth-stack
```

The command verifies that the requested `--goal` matches the active goal
manifest, reads linked plan text for work-item claim boundaries, and writes:

- `docs/handoffs/<date>-<goal-id>-closeout.md`
- `.codex/goals/archive/<date>-<goal-id>.toml`

Use `--date YYYY-MM-DD` to make output filenames deterministic. Tests and
automation can also pass `--handoff-output` and `--archive-output` to write to
fixture or scratch paths.

The generated handoff includes objective, end state, landed work items, proof
commands, receipt refs, claim boundaries, remaining work, and a generated
boundary. The archived TOML copy is written with `status = "archived"` so the
goal history can be preserved without relying on chat history. The command does
not call GitHub, inspect PR state, mutate provider records, change branch
protection, move release authority, or prove runtime product behavior.

## Override workspace root

For tests / development outside the repo:

```bash
cargo xtask --workspace-root /tmp/fixture-workspace policy-report
```

Or set `SHIPLOG_XTASK_WORKSPACE_ROOT`.

## Design

`xtask` is the Rust-native control plane for checks that otherwise
drift into shell. Shell scripts remain as compatibility wrappers until
xtask parity is proven; the shell-script-as-wrapper rule is documented
in [`docs/FILE_POLICY.md`](FILE_POLICY.md).

Cross-platform parity (Windows native `bash`/`python3` replacement) is
a follow-up release concern — it is part of the eventual Rust port of
the existing scripts, not of `#143` (the foundation).

## Adding a new command

1. Add a module under `xtask/src/tasks/<name>.rs` exposing
   `pub fn run(workspace_root: &Path) -> anyhow::Result<()>`.
2. Register the module in `xtask/src/tasks/mod.rs`.
3. Add a `Command::<Name>` variant in `xtask/src/cli.rs` and dispatch
   it in `Cli::run`.
4. Write unit tests in the same module + an integration test in
   `xtask/tests/cli.rs`.
5. Document the command in this doc.

## See also

- [`docs/ci/policy-ledgers.md`](ci/policy-ledgers.md) — `policy/` architecture
- [`docs/FILE_POLICY.md`](FILE_POLICY.md) — shell-script-as-wrapper rule
- [`policy/README.md`](../policy/README.md) — ledger inventory
