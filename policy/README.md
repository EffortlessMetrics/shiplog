# policy/

Machine-readable policy ledgers for shiplog. Skeletons added in PR #141; see
the master rollout doc at
[`docs/ci/rust-1.95-rollout.md`](../docs/ci/rust-1.95-rollout.md).

## Files

| Ledger | Owner | Loaded by |
|--------|-------|-----------|
| `ci-budget.toml` | release/ci | PR plan (#146); LEM forecasting and budget tiers |
| `ci-lanes.toml` | release/ci | PR plan (#146); per-workflow lane assignments |
| `ci-risk-packs.toml` | release/ci | PR plan (#146); path-pattern → label routing |
| `ci-exceptions.toml` | release/ci | PR plan (#146); justifies expensive default-PR lanes |
| `clippy-lints.toml` | policy | Clippy ledger checker (#150) |
| `clippy-debt.toml` | policy | Clippy ledger checker (#150) |
| `clippy-exceptions.toml` | policy | Clippy ledger checker (#150) |
| `no-panic-baseline.toml` | policy | no-panic checker (#151) |
| `no-panic-allowlist.toml` | policy | no-panic checker (#151) |
| `non-rust-allowlist.toml` | policy | file-policy checker (#149) |
| `non-rust-debt.toml` | policy | file-policy checker (#149) |
| `generated-allowlist.toml` | policy | file-policy checker (#149) |
| `executable-allowlist.toml` | policy | file-policy checker (#149) |
| `workflow-allowlist.toml` | policy | file-policy checker (#149) |
| `dependency-surface-allowlist.toml` | release | file-policy checker (#149) |
| `process-allowlist.toml` | policy | file-policy checker (#149) |
| `network-allowlist.toml` | policy | file-policy checker (#149) |
| `ripr-suppressions.toml` | policy | ripr lane (#153) |

## Common receipt fields

Every ledger uses the same header:

```toml
schema_version = 1
policy = "<ledger-name>"
owner = "EffortlessMetrics"
status = "advisory"
```

Every entry uses the schema documented in
[`docs/POLICY_ALLOWLISTS.md`](../docs/POLICY_ALLOWLISTS.md):

- `id` — stable identifier referenced from source / CI
- `owner` — workspace package, or `release` / `policy` / `ci` / `docs`
- `reason` — one or two sentences; explain the situation, not the workaround
- `created` — ISO date the entry was added
- `review_after` — ISO date the entry should be revisited
- `expires` — ISO date or `"permanent"`

Plus per-ledger extension fields documented in each ledger's header comment.

## Status in this PR

All ledgers in this PR are **parse-only skeletons** (`status = "advisory"`).
No xtask, no workflows, no enforcement, no CI behavior change. The
ledgers seed obvious entries (existing workflows, scripts, debt, lint floor)
so that later PRs read from declared contracts rather than inventing policy
in code.

The xtask runner that loads these lands in PR #143; the per-ledger checkers
land in PR #149 (file policy), PR #150 (Clippy), PR #151 (no-panic), and
PR #153 (ripr). Hard enforcement is deferred to a follow-up release after
PR #148 records actuals.

## Validation

Until the xtask runner exists (PR #143), validate parse-ability with:

```bash
python3 -c "import tomllib, glob; [tomllib.load(open(p, 'rb')) for p in sorted(glob.glob('policy/*.toml'))]; print('all policy/*.toml parse OK')"
```

Python 3.11+ required (for `tomllib`).
