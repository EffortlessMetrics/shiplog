# Shiplog Release Documentation

This directory separates the **current release procedure** from historical
release evidence.

## Current procedure

Use [`release-preparation.md`](release-preparation.md) for every new release.
It owns the current order of operations, stop conditions, proof commands,
publication boundary, rollback rules, and copyable checklists.

Promotion from the development trunk into the public source repository is a
separate prerequisite for source release execution. Use
[`plans/shiplog-swarm/promotion-runbook.md`](../../plans/shiplog-swarm/promotion-runbook.md)
for that exact-source/exact-swarm transaction.

## Artifact roles

| Surface | Role |
| --- | --- |
| `release-preparation.md` | Living preparation, staging, publication, and closeout procedure. |
| `templates/` | Starting points for a new versioned readiness ledger and execution handoff. |
| `<version>-release-decision.md` | Version-specific scope and release/no-release decision. |
| `<version>-readiness.md` | Version-specific preflight and observed release evidence. |
| `../../RELEASE_HANDOFF_<version>.md` | Version-specific execution and public-state handoff. |
| `../../CHANGELOG.md` | User-visible release contents and version history. |

Versioned files and root `RELEASE_HANDOFF_*` files are historical receipts.
They may contain commands, workflow names, tools, versions, or repository state
that were correct for that release but are no longer current. **Do not copy a
historical command block into a new release.** Start from the living procedure
and current templates, then record the commands and evidence actually used.

## Authority boundary

- `EffortlessMetrics/shiplog-swarm` owns normal product, documentation, test,
  CI, policy, and shared release-candidate preparation: scope, version,
  changelog freeze, README/guides, release decision, readiness ledger, and
  handoff skeleton.
- `EffortlessMetrics/shiplog` owns the public source lineage, release tags,
  crates.io publication, GitHub Releases, signing, package-channel publication,
  and security-sensitive release credentials.
- A complete shared release candidate must be promoted from a proven exact
  swarm head into source through a regular merge-commit checkpoint before
  source release execution begins.
- Source release preparation is narrow and source-owned: validate the promoted
  candidate and adjust only explicitly authorized release-writer configuration
  when necessary. A product or shared-documentation defect goes back to swarm.

## Quick route

1. On `shiplog-swarm`, finish the release scope, shared docs, version/lockfile,
   changelog freeze, release decision, readiness ledger, and handoff skeleton.
2. Prove the exact release candidate on `shiplog-swarm/main`.
3. Promote that exact green swarm head with `cargo xtask promote`.
4. Verify and close out the regular-merge source checkpoint.
5. On current `shiplog/main`, run source preflight and merge only narrowly
   required source-owned release-writer changes.
6. Push one immutable annotated tag from the exact proven source commit.
7. Require the source Release workflow to stage and validate one exact
   four-platform candidate set while it remains non-current.
8. Publish crates.io and make the GitHub release public only after the exact tag
   workflow and terminal candidate-ready aggregate are green.
9. Verify public assets and package channels, finalize the observed release
   evidence, and back-port any source-only release documentation/workflow
   changes to swarm before normal promotion resumes.
