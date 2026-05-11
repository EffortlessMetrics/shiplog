# Risk Packs

A risk pack is a path-pattern → label / lane mapping. A PR that touches
matching paths auto-applies the listed labels and selects the listed
PR-targeted lanes.

The machine-readable mapping lives in
[`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml). This doc is
the human explanation: what each pack means, why it exists, and what it
costs.

## Why packs exist

Risk packs encode "if you change X, you probably want Y." Without packs:

- Reviewers manually apply labels every time, which they forget.
- Authors don't know which lanes are appropriate for a given change.
- High-risk surfaces (redaction, bundle integrity, schema/ids) end up
  defaulting to the same coverage as a docs change.

Packs make routing reproducible: the PR plan ([`ci-plan-json.md`](ci-plan-json.md))
shows which packs matched and which lanes were therefore selected.

## How packs combine

A PR can match multiple packs. The selected lanes are the union; the
applied labels are the union; the LEM forecast adds the unique lanes.

A PR that matches the `docs-only` pack and no other pack runs only the
PR plan + the always-on PR-fast lanes (which are still gated by the
`docs-only` skip-by-policy rule, see [`skipped-by-policy.md`](skipped-by-policy.md)).

## Pack catalog

| ID | Description | Paths (summarised) | Auto-labels | Auto-selected lanes |
|----|-------------|--------------------|-------------|---------------------|
| `redaction-privacy` | Redaction trust surface and privacy-sensitive code paths | `crates/shiplog-redact/**`, `crates/shiplog-bundle/**`, `apps/shiplog/src/**/share*.rs`, `apps/shiplog/src/**/profile*.rs` | `mutation` | `mutation_targeted`, `property` |
| `bundle-share` | Bundle integrity, share manifests, share verification | `crates/shiplog-bundle/**`, `apps/shiplog/src/**/share*.rs`, `apps/shiplog/src/**/manifest*.rs` | `mutation` | `mutation_targeted` |
| `report-schema` | Intake report schema, agent pack export, contracts/schemas | `contracts/schemas/**`, `crates/shiplog-engine/src/**/report*.rs`, `crates/shiplog-engine/src/**/intake*.rs`, `apps/shiplog/src/**/report*.rs` | `property-tests` | `property` |
| `schema-ids` | Stable contract crates (schema, ids, ports) | `crates/shiplog-schema/**`, `crates/shiplog-ids/**`, `crates/shiplog-ports/**` | `mutation` | `mutation_targeted`, `property` |
| `source-adapter` | Source ingest adapters | `crates/shiplog-ingest-*/**` | `bdd` | `bdd`, `mutation_targeted` |
| `manual-journal` | Manual event capture and journal commands | `crates/shiplog-ingest-manual/**`, `apps/shiplog/src/**/journal*.rs` | `property-tests` | `property` |
| `period-window` | Period and window resolution, named periods | `crates/shiplog-coverage/src/**/period*.rs`, `crates/shiplog-coverage/src/**/window*.rs`, `apps/shiplog/src/**/period*.rs` | `property-tests` | `property` |
| `cli-product` | CLI / user-flow surface | `apps/shiplog/**` | `bdd` | `bdd` |
| `release-package` | Release scripts, release.yml, package metadata, version alignment | `scripts/**`, `.github/workflows/release.yml`, `Cargo.toml`, `**/Cargo.toml` | `release-check` | `release_preflight` |
| `policy-workflows` | Policy ledgers, xtask, GitHub Actions workflows | `policy/**`, `xtask/**`, `.github/workflows/**` | `full-ci` | `pr_plan`, `ci_check`, `ci_check_windows`, `ci_deny`, `ci_policy` |
| `parsers-serde` | Parser surfaces (fuzz targets, `*_parse.rs`, serde derives at trust boundaries) | `fuzz/fuzz_targets/**`, `**/serde*.rs`, `**/*_parse.rs`, `**/parse*.rs` | `fuzz` | `fuzz_quick` |
| `docs-only` | Docs-only changes — skip compile-heavy lanes | `docs/**`, `README.md`, `CHANGELOG.md`, `*.md` | (none) | `pr_plan` |

## Why some packs are mutation-targeted and others are property-only

**Mutation-targeted** packs (`redaction-privacy`, `bundle-share`,
`schema-ids`, `source-adapter`, `policy-workflows` via `full-ci`) are
trust-surface changes. A regression here can ship invisible bad output;
mutation testing is the strongest available oracle-adequacy check.

**Property-only** packs (`report-schema`, `manual-journal`,
`period-window`) are schema / invariant surfaces. The strongest signal is
that invariants hold under randomized input; mutation is overkill for the
PR-level review.

**BDD** packs (`source-adapter`, `cli-product`) are user-flow surfaces.
The strongest signal is that the flows still work end-to-end.

**Release-package** routes through release preflight because its concern
is publishability, not behavior.

## What "selected lane" means

A selected lane is **opted-in for this PR**. It still has to be wired up
(the PR plan in PR #146 emits the selection; PR #155 wires the lane
routing in the actual workflows). Until those PRs land, risk-pack
selection is documented but not executed.

## Adding or changing a pack

A new risk pack:

1. Lands first in `policy/ci-risk-packs.toml` with a new `[[risk_pack]]` entry
   (per the schema in [`policy/README.md`](../../policy/README.md)).
2. Adds a row to the catalog table above.
3. Documents why the pack exists (what failure mode it addresses) and which
   lanes it selects.
4. Should not auto-apply `full-ci` unless the touched surface is genuinely
   cross-cutting; that label implies hard-tier spend.

A changed risk pack:

1. Updates the TOML.
2. Updates the catalog table.
3. Notes the change in the PR description so reviewers see the routing
   delta.

## See also

- [`policy/ci-risk-packs.toml`](../../policy/ci-risk-packs.toml) — machine-readable mapping
- [`labels.md`](labels.md) — manual label semantics
- [`ci-lane-map.md`](ci-lane-map.md) — what each lane runs
- [`skipped-by-policy.md`](skipped-by-policy.md) — what the `docs-only` skip means
- [`ci-plan-json.md`](ci-plan-json.md) — how the PR plan reports matched packs
