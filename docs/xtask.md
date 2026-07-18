# xtask

`xtask` is shiplog's Rust-native control plane for policy checks and
release-proof aggregation. It lives under [`xtask/`](../xtask/) as a
workspace member with `publish = false`.

This doc lands in PR #143 (the foundation). The four initial commands
are minimal; per-ledger checkers (file/Clippy/no-panic/ripr) and the
PR-plan/actuals lanes are added in later PRs.

See [`docs/ci/tooling-substrate.md`](ci/tooling-substrate.md) for the standard
upstream substrate and the long-term wrapper surface that `xtask` presents to
humans, agents, and CI.

## Invocation

The workspace [`.cargo/config.toml`](../.cargo/config.toml) provides an
alias:

```bash
cargo xtask <subcommand>
# expands to: cargo run --quiet -p xtask -- <subcommand>
```

## Commands

### `cargo xtask check-policy-ledgers`

Validates every `policy/*.toml` ledger for the common header and file-stem
contract. This is the preferred proof command for the Policy ledgers support
tier.

It currently delegates to the same implementation as
`cargo xtask check-policy-schemas`, which remains available as a compatibility
name for older release docs and CI references.

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

Validates [`.codex/goals/active.toml`](../.codex/goals/active.toml) and
archived manifests under [`.codex/goals/archive/`](../.codex/goals/archive/),
the Codex-facing execution-state manifests:

- the active goal manifest exists and parses;
- `.shiplog/goals/active.toml` does not exist, keeping active execution state
  in `.codex/goals/active.toml`;
- archived goal manifests parse and use `status = "archived"`;
- goal and work-item status values are recognized;
- work-item IDs are unique;
- at most one work item is `active`;
- proposal and spec references resolve to artifact IDs in
  [`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml);
- the plan reference is a safe repo-relative path, exists, and is ledgered as a
  plan artifact;
- the work-item ID is listed in the referenced implementation plan;
- ready and active work items carry proof commands;
- archived manifests do not contain open `ready` or `active` work items;
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
- proof commands start with `rtk cargo xtask ...` and reference known xtask
  subcommands.

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

Writes repo-contract inspection reports for humans and agents:

- `target/source-of-truth/graph.json`
- `target/source-of-truth/graph.md`

The report reads [`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml),
[`policy/source-only-paths.toml`](../policy/source-only-paths.toml),
[`.codex/goals/active.toml`](../.codex/goals/active.toml), and
[`docs/status/SUPPORT_TIERS.md`](status/SUPPORT_TIERS.md). It summarizes the
active goal, recommended next slice, work items, artifact links,
support-tier proof commands, graph edges, local checkout state, local merged
branch cleanup candidates, source/swarm topology, remote branch hygiene,
remote PR/issue queue hygiene, routed CI health, latest promotion PR
traceability, branch-protection settings, and promotion receipt freshness. The
recommended next slice is derived from existing report statuses so agents can
see whether to triage queues, promote swarm work, repair blocking report state,
carry receipts in the next substantive PR, or choose the next user-value slice.
The topology section classifies the full source-ahead commit range and the
changed paths. It reports raw tree identity separately from product-tree
alignment, recognizes only exact, current entries in `source-only-paths.toml`
as approved governance, and keeps unknown or product paths fail-closed as
drift. This lets a later approved source governance commit coexist with the
latest `promote/swarm-*` merge without hiding product changes. The local
checkout section reports clean/dirty status and any local
branches already merged into source or swarm so agents can clean up their own
merged branches deliberately. Its review commands check both GitHub repos for
matching PR heads before showing recent branch commits. The branch hygiene
section separates source and swarm cleanup candidates into merged and review
buckets, includes non-destructive PR/log review commands for both buckets, and
reports open PR-backed branches separately; it is an inspection receipt and
does not delete branches. The queue section
reports open PRs and issues in both repos when GitHub inspection is available.
The routed CI section reports the latest
`EM CI Routed Shiplog Rust` run on source and swarm `main`. The promotion PR
contract section checks the latest source promotion PR title, body receipts, and
merge commit against the swarm head, including both swarm and source proof, and
lists any failed receipt checks with the missing field or required phrase. The
branch-protection section checks that
`shiplog-swarm/main` requires only `Shiplog Rust Small Result`, not conditional
runner jobs. The receipt freshness section shows which latest swarm/source
promotion receipts need to be carried into the next substantive swarm PR. It
does not mutate source artifacts, rerun CI, change branch protection, open or
close PRs/issues, delete branches, or replace the dedicated validators.

### `cargo xtask pr-body`

Generates a draft pull request body from a work item in
[`.codex/goals/active.toml`](../.codex/goals/active.toml):

```bash
rtk cargo xtask pr-body --work-item promotion-cadence --output target/source-of-truth/pr-body.md
```

The command reads the active goal manifest, the linked implementation plan, and
the ledgered proposal/spec/ADR refs in
[`policy/doc-artifacts.toml`](../policy/doc-artifacts.toml). By default it
writes:

- `target/source-of-truth/pr-body.md`

The generated body includes proposal, spec, ADR, plan item, scope, non-goals,
support-tier impact, policy impact, proof commands, claim boundary, and
rollback when those fields are present in the linked plan/spec. For the proof
section, current work-item commands from `.codex/goals/active.toml` take
precedence over plan prose so active-agent PR drafts use the same commands
validated by `rtk cargo xtask check-goals`; if no current commands are recorded,
the generator falls back to the linked plan item's proof commands. When the
work item has many receipt refs, the generated body prefers compact PR receipt
refs such as `EffortlessMetrics/shiplog-swarm#135` over narrative closure notes
and reports whether it is showing a filtered compact subset or the fallback
manifest-order list. It does not infer chronology from free-form receipt text.

`pr-body` is scoped to the selected active-goal work item. It is not a generic
PR-body generator for arbitrary local diffs. If the active work item is
`promotion-cadence`, the generated body describes that governance work item; use
`rtk cargo xtask promotion-body` for source promotion PRs, and write normal one-off
swarm PR bodies from the actual diff, proof commands, and claim boundary when no
separate active work item exists.
It is a derived draft only: it does not call the GitHub API, create a PR, mutate
source artifacts, change branch protection, or replace reviewer judgment.

### `cargo xtask promotion-body`

Generates a source promotion PR body from the current source/swarm refs:

```bash
rtk cargo xtask promotion-body --output target/source-of-truth/promotion-body.md
```

By default it compares `origin/main..swarm/main`, resolves the swarm head, and
infers included swarm PRs from squash-merge commit subjects like `(#150)`. Run
IDs can be supplied when known:

```bash
rtk cargo xtask promotion-body \
  --swarm-pr-run 26803480265 \
  --swarm-main-run 26803857830 \
  --source-pr-run 26804246444 \
  --output target/source-of-truth/promotion-body.md
```

After the source promotion has merged, `origin/main..swarm/main` is normally
empty. When `origin/main` is the promotion merge and its second parent matches
`swarm/main`, the generator infers included swarm PRs from the merge parents.
Pass the final source run ID to update the proof section without restating the
swarm head or included PRs:

```bash
rtk cargo xtask promotion-body \
  --source-post-merge-run 26807984200 \
  --output target/source-of-truth/promotion-body.md
```

Use explicit inputs when regenerating a historical promotion body from a
different checkout state or when source/swarm refs no longer point at the
promotion being edited:

```bash
rtk cargo xtask promotion-body \
  --swarm-head cdda3746bca4ea3760c24ac9b987b8d1bdf00b61 \
  --included-swarm-pr EffortlessMetrics/shiplog-swarm#151 \
  --swarm-pr-run 26806111994 \
  --swarm-main-run 26806546874 \
  --source-pr-run 26806997856 \
  --source-post-merge-run 26807984200 \
  --output target/source-of-truth/promotion-body.md
```

`--included-swarm-pr` may be repeated and accepts `151`, `#151`, or
`EffortlessMetrics/shiplog-swarm#151`.

The body includes the regular-merge-commit instruction, swarm head, included
swarm PRs, swarm/source proof slots, and the release-authority claim boundary.
It is a derived draft only: it does not call GitHub, open or edit PRs, merge,
change branch protection, tag, release, publish, or move release authority.

### `cargo xtask promote`

Verifies an exact swarm head before preparing an idempotent source promotion
branch. Run it from a release-maintainer checkout with origin=shiplog and
swarm=shiplog-swarm:

    rtk cargo xtask promote --swarm-sha <exact-swarm-sha> --dry-run
    rtk cargo xtask promote --swarm-sha <exact-swarm-sha>

The command checks shared ancestry and a completed successful
Shiplog Rust Small Result run for the exact SHA, then creates or fast-forwards
promote/swarm-current-<sha> on the source remote and writes the existing
promotion-body contract. It never merges, squashes, tags, publishes, or
deploys. Open the generated PR with a regular merge commit and verify it with
cargo xtask repo-contract-report after merge.

### `cargo xtask closeout`

Generates source-of-truth closeout artifacts from
[`.codex/goals/active.toml`](../.codex/goals/active.toml):

```bash
rtk cargo xtask closeout --goal shiplog-swarm-control-plane --handoff-output target/source-of-truth/closeout.md --archive-output target/source-of-truth/active-goal-archive.toml
```

The command verifies that the requested `--goal` matches the active goal
manifest, reads linked plan text for work-item claim boundaries, and writes:

- `docs/handoffs/<date>-<goal-id>-closeout.md`
- `.codex/goals/archive/<date>-<goal-id>.toml`

Use `--date YYYY-MM-DD` to make output filenames deterministic. Agents should
write review copies to `target/source-of-truth/` with `--handoff-output` and
`--archive-output` unless the lane is intentionally archiving the active goal.

The generated handoff includes objective, end state, source/swarm state
placeholders, queue state placeholders, landed work items, proof commands,
promotion proof placeholders, receipt refs, receipt carry-forward hints, claim
boundaries, remaining work, and a generated boundary. The archived TOML copy is
written with `status = "archived"` so the goal history can be preserved without
relying on chat history. The command does not call GitHub, inspect PR state,
mutate provider records, change branch protection, move release authority, or
prove runtime product behavior.

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
