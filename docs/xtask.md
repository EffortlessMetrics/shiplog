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


### `cargo xtask badges [--check]`

Regenerates public Shields endpoint JSON under `badges/` from repo-scoped evidence. The `--check` mode writes regenerated endpoints under `target/xtask/badges/` and fails if committed endpoints drift. `RIPR_BIN` can override the `ripr` executable used for the `ripr+` badge.

### `cargo xtask ripr-pr [--check]`

Produces PR-scoped RIPR exposure evidence under `target/ripr/pr/`. The `--check` mode verifies `repo-exposure.json` is valid JSON and `repo-exposure.md` is present and non-empty.

### `cargo xtask ripr-review-comments [--check]`

Produces PR-scoped RIPR review guidance under `target/ripr/review/` using explicit `origin/main` and `HEAD` refs. The command writes `comments.json` and `comments.md`; `--check` validates the output contract without re-running `ripr`.

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
