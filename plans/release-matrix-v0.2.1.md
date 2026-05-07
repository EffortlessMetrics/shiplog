# v0.2.1 Release Matrix

This matrix is the crates.io decision record for v0.2.1. It intentionally does
not treat every workspace member from that release branch as a publish target.
It is historical; current main no longer permits production workspace crates to
rest behind `publish = false`.

## Decision Rules

- Publish crates that are stable contracts, product/trust surfaces, real
  adapters, or required optional feature boundaries.
- Do not publish dev-only tooling.
- If a published crate depends on another workspace crate, the dependency must
  either be published first or removed from the public feature surface.
- After v0.2.1, production workspace crates must be either publishable packages
  or owner modules; `publish = false` is only for dev-only tooling.

## Matrix

| Crate | Release v0.2.1? | Why | Required before yes |
|---|---:|---|---|
| `shiplog` | yes | CLI product | package proof |
| `shiplog-ids` | yes | deterministic ID contract | package proof |
| `shiplog-schema` | yes | canonical persisted schema | package proof |
| `shiplog-ports` | yes | extension traits | package proof |
| `shiplog-coverage` | yes | coverage honesty | package proof |
| `shiplog-redact` | yes | privacy trust surface | package proof |
| `shiplog-bundle` | yes | share/checksum artifacts | package proof |
| `shiplog-workstreams` | yes | curation domain | package proof |
| `shiplog-cache` | yes | supported cache facade | package proof |
| `shiplog-render-md` | yes | primary artifact renderer | package proof |
| `shiplog-render-json` | yes | machine-readable renderer | package proof |
| `shiplog-engine` | yes | orchestration API | package proof |
| `shiplog-merge` | yes | engine dependency and merge API surface | package proof |
| `shiplog-ingest-github` | yes | core GitHub adapter | package proof |
| `shiplog-ingest-git` | yes | local git adapter used by the CLI `collect git` path | refresh/run limitation documented |
| `shiplog-ingest-json` | yes | stable import format | package proof |
| `shiplog-ingest-manual` | yes | manual evidence lane | package proof |
| `shiplog-ingest-gitlab` | no | not in the v0.2.1 CLI release set | promoted after v0.2.1 as a publishable library adapter |
| `shiplog-ingest-jira` | no | not in the v0.2.1 CLI release set | promoted after v0.2.1 as a publishable library adapter |
| `shiplog-ingest-linear` | no | not in the v0.2.1 CLI release set | promoted after v0.2.1 as a publishable library adapter |
| `shiplog-cluster-llm` | yes | optional privacy/network boundary behind `llm` | package proof and fallback/privacy docs |
| `shiplog-team` | no | not in the v0.2.1 CLI release set | promoted after v0.2.1 as a publishable library surface |
| `shiplog-template` | no | not a standalone v0.2.1 contract | folded after v0.2.1 into the team owner module |
| `shiplog-testkit` | no | dev-only fixtures | `publish = false` |

## Topological Publish Order

This was the v0.2.1 manual crates.io publication order. For first publication
of a new interdependent version, dry-run and publish one crate at a time, then
resume from the next crate:

```bash
scripts/publish-dry-run.sh --from shiplog-schema
```

```text
shiplog-ids
shiplog-schema
shiplog-ports
shiplog-coverage
shiplog-cache
shiplog-redact
shiplog-bundle
shiplog-workstreams
shiplog-merge
shiplog-render-md
shiplog-render-json
shiplog-ingest-json
shiplog-ingest-manual
shiplog-ingest-git
shiplog-ingest-github
shiplog-cluster-llm
shiplog-engine
shiplog
```

GitLab, Jira, Linear, team, and template were not published in v0.2.1. That was
a release-scope decision, not a durable package category. Current main enforces
the stronger invariant with `scripts/package-boundary-audit.sh`.

## Observed Dry-Run Boundary

On the clean v0.2.1 readiness branch, `scripts/publish-dry-run.sh` proved
`shiplog-ids` and then stopped at `shiplog-schema` because crates.io had
`shiplog-ids 0.2.0` but not `shiplog-ids 0.2.1`:

```text
failed to select a version for the requirement `shiplog-ids = "^0.2.1"`
candidate versions found which didn't match: 0.2.0
```

That is a registry-state limitation of first-time multi-crate publication, not
a package-list failure. The release process must interleave dry-run and publish
steps in the topological order until each required dependency version is visible
on crates.io.

After `shiplog-ids 0.2.1` was published, the next dry-run exposed a separate
metadata issue: publishable crates must not carry versioned dev-dependencies on
the unpublished workspace-only `shiplog-testkit`. Those dev-dependencies are
path-only for workspace tests and are omitted from packaged crates.

With that fixed, `cargo publish -p shiplog-schema --dry-run` succeeds. A resumed
`scripts/publish-dry-run.sh --from shiplog-schema` then stops at
`shiplog-ports` until `shiplog-schema 0.2.1` is visible on crates.io, so the
remaining publication must continue by interleaving one dry-run and publish step
per crate in the order above.
