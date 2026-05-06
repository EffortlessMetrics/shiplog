# shiplog API Surface

shiplog is module-first. Public crates exist for product contracts, trust
surfaces, adapter boundaries, and heavy optional boundaries. Internal SRP seams
start as modules inside the owning crate.

The product is the CLI workflow:

```text
collect -> curate -> render
```

The artifact contract is:

```text
packet + ledger + coverage + bundles
```

The mental model is: human narrative, machine receipts.

## Primary Product

- `shiplog`: CLI product and composition root.
- `shiplog-engine`: orchestration API for collect, render, refresh, import, and
  merge flows.
- `shiplog-testkit`: dev-only fixtures and BDD helpers. It is not published.

## Stable Public Contracts

These crates are the hard integration surface. Keep them small, documented, and
semver-disciplined.

- `shiplog-schema`: canonical event, coverage, workstream, and bundle schema.
- `shiplog-ids`: deterministic IDs used across persisted artifacts.
- `shiplog-ports`: extension traits for ingest, cluster, render, and redact.

## Supported Public Crates

These packages map to user-visible product surfaces or trust surfaces.

- `shiplog-coverage`: completeness and query-window honesty.
- `shiplog-workstreams`: suggested and curated workstream state.
- `shiplog-redact`: privacy profiles and deterministic redaction behavior.
- `shiplog-bundle`: share bundles, checksums, and profile-scoped inclusion.
- `shiplog-cache`: cache behavior exposed by real source adapters.
- `shiplog-render-md`: Markdown packet output.
- `shiplog-render-json`: JSON/JSONL output, while that format remains supported.

## Source Adapters

An adapter earns a crate when it crosses an external-system boundary or defines
a stable import format.

- `shiplog-ingest-github`
- `shiplog-ingest-git`
- `shiplog-ingest-json`
- `shiplog-ingest-manual`

GitLab, Jira, and Linear adapters may remain public only while they are usable
standalone adapters with tests and documentation:

- `shiplog-ingest-gitlab`
- `shiplog-ingest-jira`
- `shiplog-ingest-linear`

## Optional Boundaries

These crates are public only when the feature boundary is real enough to carry
its own dependency and privacy cost.

- `shiplog-cluster-llm`: optional network and privacy boundary. It must stay
  feature-gated and off by default.
- `shiplog-team`: optional team aggregation surface.
- `shiplog-merge`: public only while multi-source merge remains a supported API.
- `shiplog-template`: public only when the packet template format is a stable
  user contract.

## Internal Module Families

These are implementation seams, not package promises. They should live under the
nearest owning crate unless a later PR deliberately promotes them.

- Redaction internals: `shiplog-redact::{profile, policy, projector, repo, alias}`.
- Cache internals: `shiplog-cache::{key, stats, expiry, sqlite}`.
- Date windows: `shiplog-coverage::windows`.
- Output layout: `shiplog-bundle::layout`.
- Team phases: `shiplog-team::{core, aggregate, render}`.
- Workstream phases: `shiplog-workstreams::{cluster, layout, receipt_policy}`.
- LLM prompt/parse helpers: `shiplog-cluster-llm::{prompt, parse}`.
- Manual event parsing: `shiplog-ingest-manual::events`.
- Receipt rendering helpers: `shiplog-render-md::receipt`.
- Generic data structures, queues, counters, random helpers, middleware,
  parsers, normalizers, paths, URLs, encoders, and other utility code.

## Promotion Rule

Start every new boundary as a module. Promote it to a crate only when at least
one of these is true:

- It is a stable contract that external users should import directly.
- It is a trust surface shiplog users need to inspect or reuse independently.
- It crosses a real adapter boundary.
- It isolates a heavy optional dependency, network surface, or privacy risk.

If two public crates need the same shared type, move the shared contract into
`shiplog-schema`, `shiplog-ports`, or `shiplog-ids`. Do not hide production
dependencies in unpublished sibling crates.

## Feature Flags

Feature flags must represent user-visible capabilities or dependency
boundaries, not internal file layout.

Use flags like these when the capability exists:

- `llm`
- `team`
- `gitlab`
- `jira`
- `linear`
- `templates`
- `plugin` only after a plugin API is promoted.

Avoid flags named after internal implementation parts such as cache keys,
redaction policy, or output layout.

## Current State

The weak implementation-carrier families have been folded into owner crates:
redaction, cache, date windows, LLM prompt/parse, manual events, output layout,
receipt formatting, team phases, and workstream phases.

`shiplog-template` remains conditional: it should be published only if packet
templates are treated as a stable user contract for this release.

New implementation seams should start as owner modules and should not become
workspace crates unless this document deliberately promotes them.
