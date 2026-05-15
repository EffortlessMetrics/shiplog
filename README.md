# shiplog

[![CI](https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml/badge.svg)](https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml)
[![Codecov](https://codecov.io/gh/EffortlessMetrics/shiplog/branch/main/graph/badge.svg)](https://codecov.io/gh/EffortlessMetrics/shiplog)
[![crates.io](https://img.shields.io/crates/v/shiplog.svg)](https://crates.io/crates/shiplog)
[![docs.rs](https://docs.rs/shiplog/badge.svg)](https://docs.rs/shiplog)
[![MSRV](https://img.shields.io/badge/MSRV-1.95-blue.svg)](https://blog.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

> Compile code, ticket, review, local git, JSON, and manual work evidence into defensible review packets -- with receipts.

## Why shiplog?

Performance reviews ask "what did you ship?" shiplog answers that question with evidence.

**Receipts-first.** Every claim in a shiplog packet traces back to fetched data -- PRs, merge requests, reviews, tickets, local commits, imported artifacts, and manual entries. Missing data is explicitly flagged in a coverage manifest, never silently omitted.

**Coverage-first.** shiplog tracks what it queried and what might be incomplete. The coverage manifest documents API query windows, pagination limits hit, and gaps -- so you know exactly what the packet does and does not cover.

**Safe sharing.** Three deterministic redaction profiles (internal, manager, public) let you share packets at the right level of detail. Same key + same input = same aliases across runs, powered by keyed SHA-256.

shiplog is not an analytics dashboard. It is not AI-generated narrative. It produces evidence you curate, evidence anchors you can cite, and claim prompts you answer in your own voice.

## Who is this for?

- **Individual contributors** preparing self-reviews, promo packets, or brag documents with concrete shipping receipts.
- **Tech leads** compiling structured proof of what their team shipped during a review cycle.
- **Anyone** who wants a repeatable, auditable record of professional work over a time window.

## Features

- 🔍 **Multi-source ingestion** — GitHub API, GitLab API, Jira API, Linear API, local git commits, canonical JSONL, and manual YAML events
- 📊 **Automatic workstream clustering** — repo-based by default, optional LLM-assisted semantic grouping
- ✍️ **Human-authored review scaffolds** — per-workstream evidence anchors and claim prompts without generated narrative
- 🔒 **Deterministic keyed SHA-256 redaction** — three profiles (internal / manager / public) with stable aliases
- ✅ **Coverage-first design** — every claim backed by receipts; gaps explicitly flagged in the coverage manifest
- 📦 **Zip bundling with checksums** — SHA256 manifest for integrity verification
- 🏗️ **Module-first architecture** — supported public package surface is being contracted for 0.7; internal seams live as modules

## Installation

### GitHub release binary

Download the release binary for your platform when you need shiplog quickly and
do not already have Rust installed. See [docs/install.md](docs/install.md) for
Windows, macOS, Linux, and checksum verification commands.

### From crates.io

Use Cargo when you already have a Rust toolchain:

```bash
cargo install shiplog --locked
```

Install optional LLM-assisted workstream clustering explicitly:

```bash
cargo install shiplog --locked --features llm
```

### From source

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo install --path apps/shiplog
```

Developers working on shiplog itself can run directly:

```bash
cargo run -p shiplog -- <subcommand>
```

### Prerequisites

- Rust 1.95+
- A `GITHUB_TOKEN` environment variable for GitHub ingestion
- A `GITLAB_TOKEN` environment variable for GitLab ingestion
- A `JIRA_TOKEN` environment variable for Jira ingestion
- A `LINEAR_API_KEY` environment variable for Linear ingestion

## Quick start

If the review form is already open, start here:

```bash
shiplog --version
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

`shiplog intake` is the best-effort rescue path: it creates starter local files
when needed, collects usable configured sources, records skipped sources,
renders a packet, runs review inspection, and prints next commands. The
underlying workflow is still **collect → curate → render** when you want more
control.

For a first-time, empty-directory walkthrough — commands, expected output
files, how to read the rendered pack, and what `## Source Freshness` means —
see [docs/guides/rapid-first-intake.md](docs/guides/rapid-first-intake.md).
For deadline use with an established setup, see
[docs/guides/review-deadline.md](docs/guides/review-deadline.md). For a
practical review-cycle walkthrough, see
[docs/guides/review-cycle.md](docs/guides/review-cycle.md). For the complete
`shiplog.toml` field reference, see
[docs/config-reference.md](docs/config-reference.md).
When the first packet is rough, use
[docs/guides/evidence-repair-loop.md](docs/guides/evidence-repair-loop.md) to
turn report receipts into local journal repair and a better rerun packet.

From a repository checkout, you can rehearse the rescue path without provider
tokens:

```bash
scripts/demo-review-rescue.sh --out ./out/demo-review-rescue
```

### 0. Fast review intake

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

Use this when you need a packet now. It uses `shiplog.toml` if present, creates
a minimal starter config if missing, skips unusable sources without hiding them,
and writes the packet, ledger, coverage manifest, workstream file, and bundle
manifest as soon as at least one source succeeds. It also writes
`intake.report.md` and `intake.report.json` as a durable checklist you can
reopen after the terminal scrolls away. It ends with a readiness summary: what
worked, what needs attention, and the next commands to run.
`--explain` prints source decisions and setup repair hints. Add
`--source github --source jira` to limit the intake, or `--no-open` to print
paths only. When sources are skipped, intake groups repair commands by source
and writes the same rerun guidance to `intake.report.md`. Re-running intake
creates a new run and reuses the latest prior `workstreams.yaml` when one
exists, without overwriting that earlier curation or `manual_events.yaml`.
For repeat cycles, define `[periods."review-cycle"]` in `shiplog.toml` and run
`shiplog intake --period review-cycle`.
Use `shiplog periods list` or `shiplog periods explain review-cycle` to see the
resolved window, latest matching run, and copy-ready intake/review commands.
The structured report contract is documented in
[docs/schemas/intake-report-v1.md](docs/schemas/intake-report-v1.md).
For local UI, agent, or support/debug tooling, export the validated control
surface with `shiplog report export-agent-pack --latest --output agent-pack.json`;
the derived schema is documented in
[docs/schemas/agent-pack-v1.md](docs/schemas/agent-pack-v1.md).

### 1. Initialize local files

```bash
shiplog init
shiplog config validate
shiplog config explain
shiplog doctor
```

This writes `shiplog.toml` and `manual_events.yaml` in the current directory.
Use `shiplog init --source github --source jira` to scaffold a narrower source
set, `--dry-run` to preview, and `--force` to overwrite existing scaffold files.
Use `shiplog config validate` for a token-free config and local path check,
and `shiplog config explain` to see resolved defaults and enabled source
settings. New configs include `[shiplog] config_version = 1`; run
`shiplog config migrate` on older configs to add that metadata without changing
source settings. The full config reference is in
[docs/config-reference.md](docs/config-reference.md). Copy-adaptable configs
live in `examples/configs/`, including a fixture-safe local config that can be
checked with:

```bash
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml
```

`shiplog doctor` adds token, output, identity, and safety checks before a
collection run. Use `shiplog doctor --repair-plan` when setup is broken and
you want classified, source-by-source repair commands without collecting data
or writing run artifacts.

### 2. Collect events from GitHub

For the fastest review-cycle path, collect all enabled sources from
`shiplog.toml` into one merged run:

```bash
shiplog collect multi --last-6-months
shiplog review --latest
shiplog review fixups --latest
shiplog runs list
shiplog open packet --latest
```

This reads the enabled `[sources.*]` sections from `shiplog.toml`, collects
each source, merges the resulting ledgers, and writes one packet, ledger,
coverage manifest, workstream file, and bundle manifest under `out/<run_id>/`.
Explicit date flags override the config window; otherwise `defaults.window`
is used. If one configured source fails but another source succeeds, shiplog
still writes the merged packet and records the skipped source in coverage
warnings.
Named periods from `[periods.<name>]` are also supported:

```bash
shiplog collect multi --period review-cycle
shiplog review --period review-cycle
shiplog periods explain review-cycle
```

For repeat review cycles, compare a prior run with the latest one or compare
two named periods:

```bash
shiplog runs compare --from review-2025-h2 --to latest
shiplog runs compare --from-period 2025-H2 --to-period 2026-H1
```

For weekly upkeep, use `shiplog review weekly` after a refresh or collection
run. It is read-only and highlights the latest evidence counts, source gaps,
evidence debt, and next commands before you render a packet.
Use `shiplog review fixups --latest` when you want only the highest-value
curation actions as runnable commands.
Add `--commands-only` when you want just the command lines for copy/paste.

For a single-source run, call the source directly:

```bash
shiplog collect github \
  --me \
  --last-6-months \
  --mode merged \
  --out ./out
```

This fetches merged PRs (and optionally reviews with `--include-reviews`) for the authenticated GitHub user and time window. If you omit a date window, shiplog defaults to the last six months. Use `--user <login>` instead of `--me` when you want an explicit identity. Results go into `out/<run_id>/` with a JSONL event ledger, coverage manifest, and an initial packet.

### 2. Curate workstreams

shiplog generates `workstreams.suggested.yaml` with auto-grouped workstreams based on repository. Use the workstream commands for safe edits:

```bash
shiplog workstreams list --run latest
shiplog review --latest
shiplog review fixups --latest
shiplog workstreams create --run latest --title "Platform Reliability"
shiplog workstreams rename --run latest --from "acme/platform" --to "Platform Reliability"
shiplog workstreams move --run latest --event <event_id> --to "Platform Reliability"
shiplog workstreams split --run latest --from "Platform Reliability" --to "Auth Migration" --matching "auth|oauth|sso" --create
shiplog workstreams receipts --run latest --workstream "Platform Reliability"
shiplog workstreams receipt add --run latest --workstream "Platform Reliability" --event <event_id>
shiplog workstreams receipt remove --run latest --workstream "Platform Reliability" --event <event_id>
shiplog workstreams delete --run latest --workstream "old bucket" --move-to "Platform Reliability"
shiplog workstreams validate --run latest
```

Use `journal add` when important work is not already visible in a source
adapter:

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Debugged customer import incident" \
  --workstream "Customer Reliability" \
  --impact "Prevented repeat failure before the next import window" \
  --receipt ticket=https://example.invalid/ticket/OPS-123
shiplog journal list
shiplog journal edit --id manual-2026-05-08-debugged-customer-import-incident \
  --impact "Identified the bad upstream export shape before the next import window"
```

`shiplog review` suggests a copy-paste `journal add` command when a broad
workstream has implementation receipts but no manual outcome note.
`journal edit` replaces only the fields you pass; repeated `--tag` or
`--receipt` values replace those lists for the selected entry.
`shiplog review fixups` narrows the output to the top read-only curation
actions: add context, inspect receipt anchors, split broad buckets, validate
workstreams, or repair skipped sources. Use `--commands-only` for a terse
copy-friendly list.
Use `--journal-template` when you only want factual `journal add` templates for
missing human context; replace the placeholder description before adding it to
your manual evidence.
Evidence-debt findings are packet-quality checks, not person scores: skipped
sources, partial coverage, missing or over-selected receipts, broad/misc/thin
workstreams, one-source-type workstreams, and validation issues all include a
next command.

If you prefer direct YAML editing, copy `workstreams.suggested.yaml` to
`workstreams.yaml` and edit that file.

`workstreams.yaml` is yours. shiplog never overwrites it.

### 3. Re-render the packet

```bash
shiplog render --latest
shiplog render --latest --receipt-limit 3 --appendix summary
shiplog render --latest --mode scaffold
shiplog render --latest --mode receipts
shiplog share manager --latest
shiplog open packet --latest
```

This regenerates `packet.md` using your curated workstreams while preserving
the original ledger and coverage data. Add `--redact-key <KEY>` to also
generate manager and public profile packets. On remote shells, add
`--print-path` to print the artifact path without launching an editor. Use
`--mode packet` for the default review packet, `--mode scaffold` for prompts
and evidence anchors, or `--mode receipts` for a dense audit view.
`--receipt-limit <N>` caps curated receipts shown in the main receipts section;
`--receipt-limit 0` shows none there and leaves the detail to the appendix
setting. Packet mode defaults depend on the selected profile: internal packets
keep a summary appendix, manager packets keep a summary appendix with fewer main
receipts, and public packets default to the lowest-density packet with appendix
omitted. Receipts mode defaults to a full audit appendix. Use
`--receipt-limit` and `--appendix full|summary|none` to override the density.
The default CLI packet opens with `Coverage and Limits`, listing completed
sources with event counts, skipped configured sources, and known gaps before the
summary, workstreams, receipts, and detailed coverage metadata.

For shareable outputs, `shiplog share manager --latest` and
`shiplog share public --latest` are convenience wrappers over the same
fail-closed redaction and bundle-profile behavior as `render`. They require
`--redact-key` or `SHIPLOG_REDACT_KEY` and write
`profiles/<profile>/share.manifest.json` as a postflight receipt for what was
shared, which key source was used, and the packet/zip checksums. Use
`shiplog share explain manager --latest` or
`shiplog share explain public --latest` to see what the profile includes,
removes, and blocks without writing share artifacts or requiring a key. Use
`shiplog share verify manager --latest` or
`shiplog share verify public --latest` to check the selected run, coverage
warnings, skipped sources, and redaction-key readiness without writing share
artifacts. For public packets, add `--strict` to scan the rendered public view
for obvious raw URLs and original names; this is a guardrail, not a guarantee of
perfect privacy. Use
`shiplog share verify manifest --profile manager --latest` or
`shiplog share verify manifest --profile public --latest` to re-check an
existing share receipt, packet checksum, and optional zip checksum.

## Output layout

```text
out/<run_id>/
  packet.md                       # Main self-review packet (internal profile)
  workstreams.yaml                # User-curated workstream definitions
  workstreams.suggested.yaml      # Auto-generated workstream suggestions
  ledger.events.jsonl             # Canonical, append-only event log
  coverage.manifest.json          # What was queried, completeness, gaps
  source.failures.json            # Present when configured sources failed
  bundle.manifest.json            # File checksums for integrity verification
  intake.report.md                # Review rescue checklist and next actions
  intake.report.json              # Structured intake report for tooling
  profiles/                       # Only written when a redaction key is provided
    manager/packet.md             # Redacted: keeps context, strips sensitive details
    manager/share.manifest.json   # Share receipt, key source, checksums, gaps
    public/packet.md              # Redacted: aliases repos/workstreams, strips fields
    public/share.manifest.json    # Share receipt plus public strict-scan result
```

## Commands

| Command | Description |
|---------|-------------|
| `init` | Create `shiplog.toml` and `manual_events.yaml` scaffold files |
| `doctor` | Check local config, enabled sources, token env vars, output safety, and repair plans |
| `intake` | Run best-effort review intake, render a packet, inspect it, and print next steps |
| `config validate/explain/migrate` | Validate `shiplog.toml`, print resolved settings, or add version metadata |
| `cache stats/inspect/clean` | Inspect and safely clean source API cache databases |
| `identify jira/linear` | Print provider user IDs for source configuration |
| `journal add/list/edit` | Append, inspect, correct, or add factual manual evidence from a repair ID in `manual_events.yaml` |
| `collect <source>` | Fetch events from a source and generate packet artifacts |
| `collect multi` | Collect enabled sources from `shiplog.toml` into one merged packet |
| `render` | Re-render packet from existing ledger and workstreams |
| `share manager/public` | Render a manager- or public-safe packet with fail-closed redaction |
| `share explain manager/public` | Explain included, removed, and blocked share-profile posture without writing artifacts |
| `share verify manager/public` | Preflight a share profile without writing share artifacts |
| `share verify manifest` | Verify an existing share manifest and packet/zip checksums |
| `refresh <source>` | Re-fetch events while preserving curated `workstreams.yaml` |
| `workstreams list/validate/create/rename/move/split/receipts/receipt/delete` | Inspect, validate, and safely edit workstream curation |
| `runs list/show/compare` | Discover runs, inspect their artifacts, and compare source/workstream changes |
| `runs diff` | Compare latest packet quality movement across runs |
| `review` / `review weekly` / `review fixups` | Inspect a run for skipped sources, weak curation, evidence debt, and next steps |
| `open packet/workstreams/intake-report/out` | Open run artifacts, or print their paths when opening is unavailable |
| `report validate/summarize/export-agent-pack` | Validate and summarize durable intake reports for tooling |
| `repair plan` / `repair diff` | Print receipt-derived repair guidance from intake reports and compare repair state across reruns |
| `merge` | Merge existing run directories into one packet |
| `import` | Import an existing run directory and re-render |
| `run <source>` | Legacy: collect + render in one shot |

Date-based sources accept explicit dates or a preset:

```bash
--since 2025-07-01 --until 2026-01-01
--last-6-months
--last-quarter
--year 2025
```

Explicit `--since/--until` dates take precedence. If no date window is provided,
shiplog uses `--last-6-months`.

Use `shiplog render --latest` or `--run latest` to re-render the most recent
run. `shiplog refresh --run-dir latest` refreshes the same run while preserving
curation.

GitHub and GitLab accept `--me` to infer the source user from `--token`,
`GITHUB_TOKEN`, or `GITLAB_TOKEN`. Use `--user <login>` when you want to pin the
identity explicitly.

Jira and Linear use provider-specific IDs. Use `shiplog identify jira` or
`shiplog identify linear` to print the ID you should put in `shiplog.toml` or
pass to `collect`.

### Sources

| Source | Description |
|--------|-------------|
| `github` | PR and review ingestion from GitHub API (with adaptive slicing and SQLite cache) |
| `gitlab` | Merge request and review-note ingestion from GitLab API (supports self-hosted instances) |
| `jira` | Issue ingestion from Jira API; `--user` is the assignee JQL value, and `--auth-user` is available when Basic Auth uses a different email/username |
| `linear` | Issue ingestion from Linear API; `--user-id` is the Linear user UUID, with upstream date/status/project filtering |
| `git` | Local git commit ingestion for `collect git`, `refresh git`, and `run git` |
| `json` | Import from canonical JSONL event files |
| `manual` | Ingest non-GitHub work from a YAML events file |

### Examples

```bash
# Collect every enabled source from shiplog.toml into one merged packet
shiplog collect multi \
  --config shiplog.toml \
  --last-6-months

# Check a fixture-safe example config without source tokens
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml

# Inspect source API caches without touching packet outputs
shiplog cache stats --out ./out
shiplog cache inspect --out ./out --source github
shiplog cache clean --out ./out --source github
shiplog cache clean --out ./out --source jira --older-than 30d --dry-run

# Refresh receipts while keeping curated workstreams
shiplog refresh github \
  --me \
  --last-6-months \
  --run-dir out/20260115_143022 \
  --out ./out

# Import from pre-built JSON artifacts
shiplog collect json \
  --events ./ledger.events.jsonl \
  --coverage ./coverage.manifest.json \
  --out ./out

# Collect commits from a local git repository
shiplog collect git \
  --repo . \
  --year 2025 \
  --author you@example.com \
  --out ./out

# Refresh local git receipts while preserving curated workstreams
shiplog refresh git \
  --repo . \
  --year 2025 \
  --run-dir out/20260115_143022 \
  --out ./out

# Collect merge requests from GitLab
shiplog collect gitlab \
  --me \
  --last-6-months \
  --state merged \
  --instance gitlab.com \
  --include-reviews \
  --out ./out

# Collect assigned issues from Jira
shiplog identify jira \
  --instance company.atlassian.net \
  --auth-user you@example.com

shiplog collect jira \
  --user 712020:account-id \
  --auth-user you@example.com \
  --last-quarter \
  --status done \
  --instance company.atlassian.net \
  --out ./out

# Collect assigned issues from Linear
shiplog identify linear

shiplog collect linear \
  --user-id 9cfb482a-81e3-4154-b5b9-2c805e70a02d \
  --last-quarter \
  --status done \
  --project OPS \
  --out ./out

# Collect manual (non-GitHub) events
shiplog collect manual \
  --events ./manual_events.yaml \
  --user your-username \
  --year 2025 \
  --out ./out

# Merge existing source runs into one packet
shiplog merge \
  --input out/github-run \
  --input out/jira-run \
  --input out/manual-run \
  --out ./out \
  --conflict prefer-most-recent
```

`shiplog cache` commands operate only on known source API databases under
`<out>/.cache` or `--cache-dir`: `github-api-cache.db`, `gitlab-api-cache.db`,
`jira-api-cache.db`, and `linear-api-cache.db`. `cache clean` removes expired
entries by default, `--older-than 30d` removes older entries, and `--all`
requires `--yes`. It does not delete packet artifacts, ledgers, coverage
manifests, bundles, or workstream files.

## Redaction and profiles

Internal packets do not require a redaction key. Manager and public packets fail
closed unless you provide `--redact-key` or set `SHIPLOG_REDACT_KEY`.
If a share profile is requested without a key, the CLI prints the required env
var and the `--bundle-profile` command to rerun.

Generate redacted packets by providing a key:

```bash
shiplog render --latest --redact-key my-stable-secret
```

The key drives deterministic keyed SHA-256 aliasing. Same key + same input = same aliases across runs.

| Field | Internal | Manager | Public |
|-------|----------|---------|--------|
| PR titles | Visible | Visible | Stripped |
| Repository names | Visible | Visible | Aliased (e.g., `repo-a7f3`) |
| URLs | Visible | Visible | Stripped |
| Workstream names | Visible | Visible | Aliased |
| Descriptions/details | Visible | Stripped | Stripped |
| Event counts and dates | Visible | Visible | Visible |

Bundle a specific profile as a zip:

```bash
shiplog render --latest --redact-key my-stable-secret --zip --bundle-profile manager
```

## Architecture

shiplog is a module-first Rust workspace following clean architecture (ports and adapters).
The supported public package surface is being contracted for 0.7. Internal
implementation seams live under their owning packages unless a spec and ADR
promote them to supported external contracts. See [API_SURFACE.md](API_SURFACE.md)
and [docs/release/0.7-crate-surface.md](docs/release/0.7-crate-surface.md) for
the package boundary doctrine.

### Data flow

```text
Sources ─→ Ingestors ─→ Normalizer ─→ Clusterer ─→ Renderer ─→ Output
  │             │             │             │            │
  └── Cache     └── Schema    └── Workstreams └── Redact  └── Bundle
```

Events flow left-to-right through well-defined ports. Product boundaries are tested at the
crate level; smaller implementation seams stay as owner modules unless they earn an
external contract. Adapters depend on ports and schema, never the reverse.

### Crate map

```text
                    +------------------+
                    |   shiplog (CLI)  |
                    +--------+---------+
                             |
                    +--------v---------+
                    |  shiplog::engine   |
                    |  (orchestration)  |
                    +--------+---------+
                             |
          +------------------+------------------+
          |                  |                  |
  +-------v------+  +-------v------+  +--------v-------+
  | Ingest       |  | Process      |  | Output         |
  | - github     |  | - workstreams|  | - markdown     |
  | - git        |  | - redact     |  | - artifact json|
  | - json       |  | - cluster-llm|  | - bundle       |
  | - manual     |  | - team/merge |  |                |
  +--------------+  +--------------+  +----------------+
          |                  |                  |
          +------------------+------------------+
                             |
                    +--------v---------+
                    |   Foundation     |
                    | ports, schema,   |
                    | ids, coverage,   |
                    | cache            |
                    +------------------+
```

### Workspace Surface During 0.7 Contraction

| Surface | Role |
|-------|------|
| `shiplog` | Supported CLI entrypoint and default public package surface |
| `shiplog::schema` | Internal typed schema support; JSON schemas are the public machine contract |
| `shiplog::engine` | Internal orchestration support during contraction |
| `shiplog::ports` | Internal trait definitions until a plugin/API ADR exists |
| `shiplog::ids` | Internal deterministic ID helpers during contraction |
| `shiplog::workstreams` | Internal workstream clustering and curated YAML workflow |
| `shiplog-merge` | Internal multi-source merge helpers |
| `shiplog::cluster_llm` | Internal optional LLM-assisted semantic clustering behind the `llm` feature |
| `shiplog-team` | Internal team aggregation support |
| `shiplog::redact` | Internal deterministic keyed SHA-256 redaction support |
| `shiplog-testkit` | Shared test fixtures (not published) |

Markdown rendering now lives under `shiplog::render::md`. Source adapters live
under `shiplog::ingest`, and cache/freshness support lives under
`shiplog::cache`. Coverage and date-window support lives under
`shiplog::coverage`. Bundle and checksum support lives under
`shiplog::bundle`. JSON artifact writing lives under the engine artifact writer
for this contraction slice. Redaction support lives under `shiplog::redact`.
Internal implementation seams live under
owner packages as modules. New boundaries start as modules and are promoted to
public crates only when they pass the public crate test in `SHIPLOG-SPEC-0004`.

## LLM clustering

By default, shiplog clusters events by repository. With the `llm` feature, you can enable semantic clustering via an OpenAI-compatible API:

```bash
# Install with LLM support
cargo install shiplog --features llm

# Use LLM clustering during collection
shiplog collect github \
  --me \
  --last-6-months \
  --out ./out \
  --llm-cluster \
  --llm-api-key $SHIPLOG_LLM_API_KEY
```

LLM clustering is feature-gated and off by default. It falls back to repository-based clustering on failure. See `--llm-api-endpoint` and `--llm-model` for endpoint configuration.

## Documentation

- [CHANGELOG](CHANGELOG.md) -- Release history and migration notes.
- [API_SURFACE](API_SURFACE.md) -- Public crate boundary doctrine.
- [ROADMAP](ROADMAP.md) -- What is planned, what is next, and what is out of scope.
- [CONTRIBUTING](CONTRIBUTING.md) -- Setup, conventions, and how to submit changes.
- [docs.rs/shiplog](https://docs.rs/shiplog) -- API documentation for the supported CLI package.

### Policy doctrine (v0.5.0+)

- [docs/CLIPPY_POLICY.md](docs/CLIPPY_POLICY.md) -- Lint ledger, exception ledger, and how to add a justified `#[expect(...)]`.
- [docs/NO_PANIC_POLICY.md](docs/NO_PANIC_POLICY.md) -- Panic-family baseline (no-new-debt enforcement) and the procedure to reset it in a dedicated PR.
- [docs/FILE_POLICY.md](docs/FILE_POLICY.md) -- Non-Rust file allowlist, generated-file allowlist, executable bit, workflow, dependency-surface, process-policy, and network-policy companions.
- [docs/POLICY_ALLOWLISTS.md](docs/POLICY_ALLOWLISTS.md) -- Common header schema and the per-checker semantics (advisory vs blocking-allowlist).

## For contributors

We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions, coding conventions, and how to submit changes.

```bash
# Quick dev loop
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps

# Policy gates (advisory by default; release-prep runs them in --mode blocking-allowlist)
cargo xtask check-policy-schemas
cargo xtask policy-report
```

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.
