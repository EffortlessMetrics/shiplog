# Shiplog Release Preparation and Execution

**Status:** active living procedure
**Authority:** `shiplog-swarm` prepares and proves the complete shared
candidate; `shiplog` owns source execution, tags, publication, and public
release state.

This is the canonical procedure for a new Shiplog release. Versioned readiness,
decision, and handoff files are evidence for one completed release; they are not
standing instructions.

## Operating invariants

1. Normal product, shared documentation, tests, CI, policy, package metadata,
   version changes, changelog changes, and release-candidate preparation land in
   `EffortlessMetrics/shiplog-swarm`.
2. `EffortlessMetrics/shiplog` remains the public source and release authority.
   It owns release tags, crates.io publication, GitHub Releases, signing,
   package-channel publication, and security-sensitive release credentials.
3. Source receives the complete proven candidate through a regular merge-commit
   promotion PR. Never squash a source promotion.
4. Source release work begins only after the exact shared candidate has been
   promoted. It is limited to validation, tagging, publication, and narrowly
   authorized source-owned writer configuration.
5. A product, version, package, changelog, README, guide, test, or shared CI
   defect found on source returns to swarm, is proved there, and is promoted
   again. Source is not a second release-preparation lane.
6. The release tag is immutable. Never force-move or reuse a failed tag; fix the
   defect through swarm and cut the next patch version.
7. One release attempt has one exact staged candidate set. Every platform test,
   checksum, readiness receipt, and publication decision must identify the same
   tag, source commit, artifact manifest, and artifact digests.
8. A skipped or unavailable optional check is not claimed as executed proof.
9. Release claims come from merged behavior and observed artifacts, not commit
   subjects, PR-body assertions, historical command blocks, or a local rebuild.

## Phase 0 — Define the candidate on swarm

Choose the release version and theme from the user-visible change set, not from
control-plane activity alone.

Before candidate freeze:

- identify the user-facing reason for the release;
- decide the intended semantic-version increment;
- list included and explicitly deferred user-visible changes;
- identify installer, package-channel, signing, or platform-trust impact;
- create `docs/release/X.Y.Z-release-decision.md` from
  [`templates/release-decision.md`](templates/release-decision.md);
- create `docs/release/X.Y.Z-readiness.md` from
  [`templates/release-readiness.md`](templates/release-readiness.md); and
- create root `RELEASE_HANDOFF_X.Y.Z.md` from
  [`templates/release-handoff.md`](templates/release-handoff.md).

Keep these files in `shiplog-swarm`. Their pre-publication form records intended
scope, required proof, and placeholders for observed source/tag/public state.

Do not bump versions merely to make a candidate feel real. Finish product scope,
README/guides, changelog curation, and the release decision first. Make the
version/changelog freeze late in the final shared release-preparation PR.

## Phase 1 — Finish and freeze the shared candidate on swarm

Release preparation starts on current `shiplog-swarm/main`.

### Content and documentation review

- Curate `CHANGELOG.md` from merged behavior. Include user-visible capability,
  meaningful compatibility changes, and important fixes; omit internal churn.
- Update README installation, first-use, recurring-use, and command examples
  affected by the release.
- Update guides, configuration reference, support tiers, claim boundaries, and
  package-channel instructions before freezing release notes.
- Verify every changelog and release-note statement against the actual command
  path. Narrow any unproved security, platform, signing, or availability claim.
- Remove stale future-tense wording from current docs without rewriting
  historical release receipts.
- Confirm the release still has one coherent user-facing theme after deferrals.

### Final shared release-preparation PR

After scope and documentation are stable, use one focused swarm PR to:

- set the workspace/package version to `X.Y.Z`;
- align workspace package entries in `Cargo.lock`;
- freeze the curated changelog section as `X.Y.Z` with the intended release
  date and create a fresh empty `[Unreleased]` section;
- update the documentation-only `publish.release` marker when appropriate,
  without misrepresenting it as an executable gate;
- finalize the release decision and pre-publication readiness state;
- prepare the handoff skeleton with exact fields still marked pending; and
- make any last shared README, installer, guide, test, or verification changes.

Do not make these shared changes directly in `EffortlessMetrics/shiplog`.

### Queue and repository state

Both repositories may contain deliberately deferred work, but the candidate
must not be ambiguous.

```bash
git fetch origin --prune
git fetch swarm --prune

gh pr list --repo EffortlessMetrics/shiplog-swarm --state open --limit 50
gh issue list --repo EffortlessMetrics/shiplog-swarm --state open --limit 50
gh pr list --repo EffortlessMetrics/shiplog --state open --limit 50
gh issue list --repo EffortlessMetrics/shiplog --state open --limit 50
```

For every open item, record one of:

- included before candidate freeze;
- explicitly deferred and non-blocking;
- superseded/duplicate and closed; or
- release blocker.

Do not promote while a required-check failure, source mutation incident,
release-blocking PR, or source/shared candidate mismatch has an unexplained
disposition.

### Swarm proof

Run the contributor and repository-contract proof from a clean tracked checkout.
Known protected agent workspace artifacts may remain only when the contract
report classifies them explicitly; tracked edits and unknown untracked paths
remain blocking.

```bash
cargo xtask ci-small
cargo xtask check-policy-ledgers
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-automation-authority --repository-role swarm
cargo xtask promotion-state --check
cargo xtask repo-contract-report

bash scripts/package-version-audit.sh
bash scripts/package-boundary-audit.sh
bash scripts/package-proof.sh
bash scripts/publish-dry-run.sh

git diff --check
```

On Windows, run the Bash package scripts through WSL or Git Bash. Hosted Linux
CI remains the authority for their normal execution.

Confirm independently:

- every publishable workspace package reports `X.Y.Z`;
- `policy/publish-allowlist.toml` contains only intended public packages;
- `cargo package --list` contains no private, generated, or unintended files;
- `cargo publish --dry-run --locked` succeeds for the allowlisted order;
- the exact candidate head has a successful routed
  `Shiplog Rust Small Result`; and
- version, changelog, README/guides, decision, readiness, and handoff all belong
  to that same exact swarm commit.

The required aggregate is the authority; optional smoke lanes are supplementary
evidence.

## Phase 2 — Promote the exact release candidate

Follow
[`plans/shiplog-swarm/promotion-runbook.md`](../../plans/shiplog-swarm/promotion-runbook.md).
The canonical entrypoint is `cargo xtask promote`; do not hand-build a raw
`swarm/main` source branch.

### Read-only planning

Resolve the current exact head after fetching both remotes:

```bash
git fetch origin --prune
git fetch swarm --prune
swarm_head="$(git rev-parse swarm/main)"
mkdir -p target/source-of-truth

cargo xtask promote --swarm-sha "$swarm_head" --dry-run \
  > target/source-of-truth/promote-plan-1.json
cargo xtask promote --swarm-sha "$swarm_head" --dry-run \
  > target/source-of-truth/promote-plan-2.json
cmp target/source-of-truth/promote-plan-1.json \
    target/source-of-truth/promote-plan-2.json
```

PowerShell equivalent:

```powershell
git fetch origin --prune
git fetch swarm --prune
$swarmHead = (git rev-parse swarm/main).Trim()
New-Item -ItemType Directory -Force target/source-of-truth | Out-Null

cargo xtask promote --swarm-sha $swarmHead --dry-run |
  Set-Content -NoNewline target/source-of-truth/promote-plan-1.json
cargo xtask promote --swarm-sha $swarmHead --dry-run |
  Set-Content -NoNewline target/source-of-truth/promote-plan-2.json

if ((Get-FileHash target/source-of-truth/promote-plan-1.json).Hash -ne
    (Get-FileHash target/source-of-truth/promote-plan-2.json).Hash) {
  throw "promotion dry-runs are not deterministic"
}
```

Review the structured plan, not only the exit code. Confirm:

- `source_head`, `swarm_head`, and merge base are the expected exact commits;
- the routed workflow and terminal aggregate belong to that exact swarm SHA;
- the included merged swarm PR receipt list is complete;
- every differing path has an exact source/swarm tree entry, effect, and basis;
- every source-retained path has current bounded source authority;
- every exceptional take-swarm path has current bounded transition authority;
- the deterministic overlay SHA and source branch/PR action are explicit; and
- neither dry-run created a branch, PR, worktree residue, durable object, or
  source mutation.

A non-zero dry-run is a repair queue. Do not bypass it with an old
`--source-ref`, a raw branch push, a guessed receipt, or `--allow-historical`.
Historical mode is for explicit historical diagnosis; it is not permission to
prepare a PR against a different current source head.

### Prepare, prove, and merge the source checkpoint

After the exact dry-run is deterministic and fully reviewed:

```bash
cargo xtask promote --swarm-sha "$swarm_head"
```

A real execution must re-resolve live state, require source merge control,
create/update/reuse one deterministic source-local promotion branch and PR,
preserve current source-owned release/governance paths through the exact
overlay, emit the machine receipt, and stop before merge/tag/publication.

Run the command again against unchanged state. It must reuse the same branch,
PR, overlay, title, body, and receipt identity rather than create duplicates.

Review the actual source PR diff and exact-head source CI. Its body must record
the exact swarm head, included swarm PRs, swarm proof, source proof, regular
merge instruction, rollback, and the release-authority claim boundary.

Merge with a regular merge commit, never squash:

```bash
gh pr merge <number> --repo EffortlessMetrics/shiplog --merge --delete-branch
```

Then verify the landed transaction:

```bash
git fetch origin --prune
git fetch swarm --prune
cargo xtask promote --swarm-sha "$swarm_head" --verify-only
cargo xtask repo-contract-report
```

Require the exact two-parent checkpoint, matching overlay identities, exact
source post-merge routed CI, no unexplained source product commits, only current
approved source governance, and a closed-out bounded promotion state.

The required closeout records one landed transaction. It is distinct from
opening repeated receipt-only PRs merely to chase a moving pending range.

## Phase 3 — Validate source release execution

After promotion, current `shiplog/main` should already contain the version,
changelog freeze, shared docs, decision, readiness ledger, handoff skeleton,
package state, and candidate behavior.

Do **not** create a source release-prep PR merely to repeat those changes.

A focused source PR is allowed only when the release requires a narrowly
source-owned change, such as:

- release-writer permissions or source-only workflow projection;
- source-specific signing/notarization configuration;
- source-only package-publication credentials or protected environment wiring;
- source-only release notes that cannot exist on the verification-only swarm
  workflow; or
- an explicitly authorized release-governance correction.

It must not change the shared product, version, lockfile, changelog, README,
guides, tests, or ordinary CI. If one of those needs correction, stop, fix it on
swarm, promote a new exact candidate, and restart source validation.

### Source preflight

Run from current source main, or from the narrow source-owned release-execution
PR when one is required:

```bash
cargo xtask ci-small
cargo xtask check-policy-ledgers
cargo xtask check-doc-artifacts
cargo xtask check-goals
cargo xtask check-support-tiers
cargo xtask check-workflows --mode blocking-allowlist
cargo xtask check-file-policy --mode blocking-allowlist
cargo xtask check-automation-authority --repository-role source
cargo xtask promotion-state --check
cargo xtask repo-contract-report

bash scripts/package-version-audit.sh
bash scripts/package-boundary-audit.sh
bash scripts/package-proof.sh
bash scripts/publish-dry-run.sh
bash scripts/check-release-hold.sh vX.Y.Z

git diff --check
```

Confirm:

- source product/package/version/changelog state matches the exact promoted
  candidate;
- source automation authority is `source`, not `swarm`;
- release workflow write permission is narrowly job-scoped;
- package proof and allowlisted publish dry-run succeed;
- the release-hold guard accepts the exact intended tag; and
- exact source PR and post-merge checks are green when a source-owned PR was
  required.

Do not tag from an unmerged PR head. The release source commit is the exact
promoted source main or the exact merged result of the narrow source-owned
execution PR.

## Phase 4 — Stage the exact tagged candidates

The swarm candidate contract is implemented and proven by the staged-candidate
workflow on #391. The source-owned counterpart still needs to project that
contract into the source release writer; this procedure does not claim an
exact-tag run or public release until that source work and live evidence exist.

Tag only the exact proven source commit recorded in the readiness ledger. Do
not tag whichever source checkout happens to be current:

```bash
approved_sha="<40-character readiness SHA>"
tag="vX.Y.Z"

if [[ ! "$approved_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "approved readiness SHA must be a full 40-character commit SHA" >&2
  exit 1
fi

git fetch origin "$approved_sha"

test -z "$(git status --porcelain --untracked-files=no)"

git switch --detach "$approved_sha"

test "$(git rev-parse HEAD)" = "$approved_sha"

if git rev-parse --quiet --verify "refs/tags/$tag" >/dev/null; then
  echo "tag already exists: $tag" >&2
  exit 1
fi

git tag -a "$tag" "$approved_sha" \
  -m "Shiplog $tag"

test "$(git rev-list -n 1 "$tag")" = "$approved_sha"
git push origin "$tag"
```

Stop before tagging when the tracked checkout is dirty or the approved SHA is
unavailable. If source has advanced after readiness, do not silently adopt the
new head; return to swarm and produce a new approved candidate. If the tag
already exists, do not move or reuse it. If the immutable tag or candidate
fails, repair through swarm and cut the next version. The tag push invokes the
source `Release` workflow. A manual dispatch is only for an existing explicit
semver tag and requires the owner-approval input; it does not replace the tag
or authorize an untagged build.

The workflow must keep the candidate non-current while it proves one immutable
staged set. A draft GitHub release, retained workflow-artifact bundle, or other
source-authorized equivalent may be used, but every later lane must consume the
same candidate manifest and artifact digests.

Require successful completion of:

- `Release Preflight`;
- package proof and allowlisted publish dry-run;
- Linux x86-64 binary build;
- macOS Intel binary build;
- macOS Apple Silicon binary build;
- Windows x86-64 binary build;
- staged candidate manifest, checksums, and artifact digests;
- draft GitHub Release creation and asset upload when that is the staging
  mechanism;
- release validation;
- first-use acceptance on all four supported targets;
- release-mode integration tests;
- deliberate checksum and executable/first-use negative controls; and
- one terminal `Release Candidate Ready` aggregate depending on every required
  candidate job.

Every job, artifact, manifest, and draft release must point at the same immutable
tag SHA. No acceptance lane may use a workspace binary, `cargo run`, or a local
rebuild as the binary under test.

If any exact-tag gate fails:

- keep the candidate non-current and any GitHub release draft;
- do not publish crates.io;
- do not make the GitHub release public;
- do not move or reuse the tag; and
- fix the defect through swarm and cut the next patch version.

## Phase 5 — Publish

Only after all staged-artifact lanes and `Release Candidate Ready` are green,
publish the crate from a detached checkout of that same immutable tag:

```bash
git fetch origin --prune --tags
git switch --detach vX.Y.Z
test "$(git rev-parse HEAD)" = "$(git rev-list -n 1 vX.Y.Z)"

cargo publish -p shiplog --locked
gh release edit vX.Y.Z --draft=false --latest
```

Do not publish from a moving `main`, an unmerged branch, or a local rebuild that
is not the exact tag exercised by the source Release workflow.

Verify public state:

- crates.io shows the intended version and package metadata;
- `cargo install shiplog --version X.Y.Z --locked` installs successfully;
- the GitHub release is public, non-prerelease unless explicitly intended, and
  marked latest when appropriate;
- all four binaries and `SHA256SUMS.txt` are present;
- downloaded binaries report `shiplog X.Y.Z`;
- versionless installer paths resolve the public assets; and
- Homebrew and Scoop updates use the final public asset hashes and pass native
  package validation.

Claim signing, notarization, SmartScreen reputation, or other platform-trust
proof only when the corresponding lane exists and passed. Checksums alone are
not a signing claim.

## Phase 6 — Record observed release evidence on swarm

After public verification, complete the versioned readiness ledger and handoff
on a focused swarm release-closeout PR. Replace placeholders with observed:

- exact promoted swarm and source commits;
- exact tag and source Release workflow run;
- staged candidate manifest and artifact digests;
- successful platform jobs and terminal candidate-ready aggregate;
- public crates.io version;
- public GitHub Release and asset list;
- first-use acceptance results;
- installer and package-channel results;
- known limitations and deferred work; and
- rollback/yank decision, if any.

Set the release decision/readiness status to shipped only after public-state
verification. The new `[Unreleased]` section was already opened during candidate
freeze and remains ready for subsequent work.

Merge and prove this closeout on swarm. Promote it to source as a coherent
release-evidence checkpoint, either immediately when public documentation must
be current or with the next substantive promotion batch. Do not edit the shared
handoff only on source and leave swarm unaware of the public result.

Any source-only release-workflow or execution change must also be reconciled
back into the swarm verification projection before normal promotion resumes.

## Abort and rollback rules

### Before the tag

Fix the candidate in swarm, rerun proof, and promote again. Abandon or rebuild
any narrow source-owned execution PR from the new source main.

### After the tag but before crates.io publication

Keep the candidate non-current and any GitHub release draft. Do not force-move
the tag. Fix through swarm and cut the next patch tag.

### After crates.io publication

Published crate bytes cannot be replaced. Use a new patch release. Yank only
when the published version should no longer be selected automatically, and
record the reason in the release handoff.

### Incorrect source promotion

Revert the regular merge commit in source and pause release work until the
divergence is understood. Never rewrite `shiplog/main` history.

## Copyable release checklist

### Shared candidate on swarm

- [ ] Release version and user-facing theme are decided.
- [ ] Included and deferred scope is explicit.
- [ ] README, guides, config docs, support tiers, and examples are current.
- [ ] Workspace/package version and `Cargo.lock` are aligned.
- [ ] Changelog `X.Y.Z` section is frozen and a new `[Unreleased]` exists.
- [ ] Release decision, readiness ledger, and handoff skeleton are prepared.
- [ ] Open PRs/issues are included, deferred, closed, or blocking explicitly.
- [ ] `cargo xtask ci-small` passes.
- [ ] Policy, docs, goals, support-tier, workflow, file-policy, and authority
      checks pass.
- [ ] Package version/boundary audits, package proof, and publish dry-run pass.
- [ ] `promotion-state --check` and `repo-contract-report` pass.
- [ ] Exact swarm `Shiplog Rust Small Result` is green.

### Promotion

- [ ] Two current-head dry-runs are byte-identical.
- [ ] Exact source/swarm/merge-base and required CI evidence are correct.
- [ ] Included swarm PR receipts are complete.
- [ ] Every path decision has exact entries and a current basis.
- [ ] Real `cargo xtask promote` creates or reuses one compatible source PR.
- [ ] Source promotion PR exact-head CI is green.
- [ ] Source promotion is regular-merged, never squashed.
- [ ] `--verify-only` and source post-merge `repo-contract-report` pass.
- [ ] Promotion manifest/current view and consumptive decisions are closed out.

### Source execution preflight

- [ ] Source shared candidate state exactly matches the promoted swarm candidate.
- [ ] No shared product/version/changelog/docs correction is being made on source.
- [ ] Any source PR is limited to source-owned release execution/governance.
- [ ] Source-role automation and release authority remain correct.
- [ ] Package version/boundary audits, package proof, and publish dry-run pass.
- [ ] Release-hold guard passes for the exact tag.
- [ ] Source PR and post-merge required checks are green when applicable.

### Tag and staged candidates

- [ ] Annotated semver tag points at the exact proven source commit.
- [ ] Tag has not been moved or reused.
- [ ] Source Release workflow targets the exact tag SHA.
- [ ] Four platform builds pass.
- [ ] One staged candidate manifest binds the tag, source commit, assets, and
      digests.
- [ ] Checksums and any draft release assets are complete.
- [ ] Release validation consumes the staged set and passes.
- [ ] Four-platform first-use acceptance consumes the same staged set and passes.
- [ ] Deliberate checksum and executable/first-use failures remain red.
- [ ] Release-mode integration tests pass.
- [ ] Terminal `Release Candidate Ready` is green.

### Publication and closeout

- [ ] crates.io publication ran from a detached checkout of the exact tag.
- [ ] GitHub release became public only after exact-tag candidate proof.
- [ ] Public assets, checksums, installers, and `--version` are verified.
- [ ] Homebrew and Scoop use final public hashes and pass native validation.
- [ ] Swarm readiness/handoff records observed run IDs and public state.
- [ ] Deferred work and limitations are explicit.
- [ ] Source-only execution/workflow changes are reconciled back to swarm.
- [ ] Observed release evidence is promoted to source on a coherent checkpoint.
- [ ] New `[Unreleased]` work can proceed without unresolved release drift.
