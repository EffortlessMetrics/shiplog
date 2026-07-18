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
| `publish-allowlist.toml` | release | package-boundary audit and publish scripts |
| `ripr-suppressions.toml` | policy | ripr lane (#153) |
| `source-only-paths.toml` | repo-infra/release | `repo-contract-report`; exact-path source governance classification |

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
`source-only-paths.toml` is a blocking exact-path governance ledger: its entries
use `path`, `classification`, and `review_after` instead of generic `id` and
`expires` fields, and an overdue `review_after` fails closed.

## Historical status

The initial ledgers were parse-only skeletons (`status = "advisory"`). They
seeded obvious entries so later PRs could read from declared contracts rather
than inventing policy in code.

The xtask runner that loads these lands in PR #143; the per-ledger checkers
land in PR #149 (file policy), PR #150 (Clippy), PR #151 (no-panic), and
PR #153 (ripr). Hard enforcement is deferred to a follow-up release after
PR #148 records actuals.

`publish-allowlist.toml` is a later blocking release ledger for the 0.7
crate-surface contraction lane. It is consumed by
`scripts/package-boundary-audit.sh`, `scripts/package-proof.sh`, and
`scripts/publish-dry-run.sh`. The boundary audit also checks that packages
disabled by the policy set `publish = false` in their Cargo manifests and that
publish-enabled packages do not depend normally on non-publishable workspace
packages or historical 0.6 implementation crates.

## Validation

Until the xtask runner exists (PR #143), validate parse-ability with:

```bash
python3 -c "import tomllib, glob; [tomllib.load(open(p, 'rb')) for p in sorted(glob.glob('policy/*.toml'))]; print('all policy/*.toml parse OK')"
```

Python 3.11+ required (for `tomllib`).
