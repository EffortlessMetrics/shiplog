<h1 align="center">shiplog</h1>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml"><img src="https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI" /></a>
  <a href="https://codecov.io/gh/EffortlessMetrics/shiplog"><img src="https://codecov.io/gh/EffortlessMetrics/shiplog/branch/main/graph/badge.svg" alt="Codecov" /></a>
  <a href="https://github.com/EffortlessMetrics/ripr/blob/main/docs/BADGE_POLICY.md"><img src="https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/EffortlessMetrics/shiplog/main/badges/ripr-plus.json" alt="ripr+" /></a>
</p>

<p align="center">
  <a href="https://github.com/EffortlessMetrics/shiplog/releases"><img src="https://img.shields.io/github/v/release/EffortlessMetrics/shiplog?sort=semver&label=release" alt="GitHub release" /></a>
  <a href="https://crates.io/crates/shiplog"><img src="https://img.shields.io/crates/v/shiplog.svg" alt="crates.io" /></a>
  <a href="https://crates.io/crates/shiplog"><img src="https://img.shields.io/crates/d/shiplog.svg?label=crates.io%20downloads" alt="crates.io downloads" /></a>
  <a href="https://docs.rs/shiplog"><img src="https://docs.rs/shiplog/badge.svg" alt="docs.rs" /></a>
</p>

<p align="center">
  <a href="https://doc.rust-lang.org/cargo/reference/manifest.html#the-rust-version-field"><img src="https://img.shields.io/badge/MSRV-1.95-blue.svg" alt="MSRV 1.95" /></a>
  <a href="LICENSE-MIT"><img src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" alt="License: MIT OR Apache-2.0" /></a>
</p>

<p align="center">
  <em>Review readiness with receipts: setup, status, intake, repair, rerun, diff, and share safely.</em>
</p>

shiplog turns work evidence into a review-readiness loop: diagnose setup,
collect receipts, inspect status, repair gaps, rerun, compare, and share
safely.

## The problem

Performance reviews ask what shipped, what mattered, and what evidence supports
it. Most people discover missing evidence too late.

shiplog keeps the loop receipt-backed:

```text
setup -> status -> intake -> repair -> rerun -> diff -> share explain
```

It is for individual contributors, tech leads, and anyone who wants a
repeatable evidence trail for self-reviews, promo packets, or brag documents.

## What works in the 0.9 candidate

| Surface | Status | Command |
|---------|--------|---------|
| Setup preflight | Ready | `shiplog doctor --setup` |
| Agent setup state | Ready | `shiplog doctor --setup --json` |
| Review cockpit | Ready | `shiplog status --latest` |
| Agent review state | Ready | `shiplog status --latest --json` |
| Evidence collection | Ready | `shiplog intake --last-6-months --explain` |
| Repair queue | Ready | `shiplog repair plan --latest` |
| Local repair | Ready | `shiplog journal add --from-repair <repair_id>` |
| Repair movement | Ready | `shiplog repair diff --latest` |
| Packet movement | Ready | `shiplog runs diff --latest` |
| Share posture | Ready | `shiplog share explain manager --latest` |

The current published release on crates.io is still 0.8.0. The 0.9 work is on
`main` as a prepared but held candidate; do not treat this README as release
approval.

## Install

Install the latest published release from crates.io:

```bash
cargo install shiplog --locked
```

Try the 0.9 candidate from a source checkout:

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo install --path apps/shiplog --locked
```

Developers can run the workspace binary directly:

```bash
cargo run -p shiplog -- <subcommand>
```

Prerequisites:

- Rust 1.95+
- Optional provider tokens when those sources are enabled:
  `GITHUB_TOKEN`, `GITLAB_TOKEN`, `JIRA_TOKEN`, or `LINEAR_API_KEY`
- `SHIPLOG_REDACT_KEY` only when rendering manager/public share packets

## First setup

Start here when the directory is empty, messy, or newly configured:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog doctor --setup --json
shiplog status --latest
```

`init --guided` writes local setup files. `doctor --setup`, `sources status`,
`doctor --setup --json`, and `status --latest` are read-only. They do not query
providers, render share packets, or mutate provider records.

## First packet

Collect usable evidence and then inspect the cockpit again:

```bash
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog open packet --latest
```

`intake` writes run artifacts under `out/<run_id>/`, including
`packet.md`, `intake.report.md`, `intake.report.json`,
`ledger.events.jsonl`, `coverage.manifest.json`, and a bundle manifest.
`status --latest` reads those receipts and tells you whether the next safe
step is repair, rerun, diff, or share explanation.

## Repair and share

When the packet is rough, stay read-first:

```bash
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
```

`repair plan` reads `intake.report.json` and separates safe local writes from
advisory work. `journal add --from-repair` writes local manual evidence only.
`repair diff` and `runs diff` prove what moved after rerun. `share explain`
is read-only and explains manager/public posture before any explicit render
command.

## Status at a glance

Use status as the cockpit for recurring review work:

```bash
shiplog status --latest
shiplog status --latest --json
```

Human output answers:

- what setup is ready, blocked, disabled, or missing credentials;
- which latest run and receipts were read;
- packet readiness and source posture;
- open repair items and whether any safe write exists;
- whether a comparable run, repair diff, or runs diff is available;
- manager/public share blockers;
- the next safe action and whether it writes files.

JSON output is the same model for agents and scripts. It uses stable keys,
deterministic ordering, no secret values, and no Markdown scraping.

## Docs map

| Need | Doc |
|------|-----|
| Install and checksums | [docs/install.md](docs/install.md) |
| First run or deadline rescue | [docs/guides/rapid-first-intake.md](docs/guides/rapid-first-intake.md) |
| Setup preflight | [docs/guides/guided-setup-doctor.md](docs/guides/guided-setup-doctor.md) |
| Recurring weekly/monthly use | [docs/guides/recurring-review-loop.md](docs/guides/recurring-review-loop.md) |
| Full GitHub history harvest | [docs/guides/github-activity-harvest.md](docs/guides/github-activity-harvest.md) |
| Evidence repair | [docs/guides/evidence-repair-loop.md](docs/guides/evidence-repair-loop.md) |
| Packet interpretation and share posture | [docs/guides/review-ready-packet.md](docs/guides/review-ready-packet.md) |
| Full configuration reference | [docs/config-reference.md](docs/config-reference.md) |
| 0.9 release readiness and hold | [docs/release/0.9.0-readiness.md](docs/release/0.9.0-readiness.md) |
| Changelog | [CHANGELOG.md](CHANGELOG.md) |

Machine-readable contracts:

- [Intake report v1](docs/schemas/intake-report-v1.md)
- [Setup readiness v1](docs/schemas/setup-readiness-v1.md)
- [Review-loop status v1](docs/schemas/review-loop-status-v1.md)
- [GitHub activity harvest receipts v1](docs/schemas/github-activity-harvest-v1.md)
- [GitHub activity report v1](docs/schemas/github-activity-report-v1.md)
- [Agent pack v1](docs/schemas/agent-pack-v1.md)

Proof receipts:

- [GitHub activity harvest completion audit](docs/product/github-activity-harvest-completion-audit.md)
- [Review-loop status transcript](docs/product/review-loop-status-transcript.md)
- [Guided setup transcript](docs/product/guided-setup-transcript.md)
- [Review-ready loop transcript](docs/product/review-ready-loop-transcript.md)
- [Setup-readiness dogfood matrix](docs/product/setup-readiness-dogfood-matrix.md)
- [Review-ready dogfood matrix](docs/product/review-ready-dogfood-matrix.md)

## What shiplog does not do

- Does not write performance-review prose.
- Does not score employees.
- Does not mutate provider records.
- Does not query providers from `doctor` or `status`.
- Does not render manager/public packets from `status` or `share explain`.
- Does not treat missing optional provider tokens as weak evidence.
- Does not make `packet.md` a machine source of truth.
- Does not require an LLM path; optional LLM clustering is feature-gated and
  off by default.

## Public surface

Single supported public crate: `shiplog`. Internal seams live as modules under
that crate unless a future public API/plugin ADR promotes a boundary. JSON
schemas under `contracts/schemas/` are the machine contracts for emitted
artifacts.

Deterministic redaction aliases are available for manager and public share
profiles. Rendering those profiles is explicit and fail-closed: provide
`SHIPLOG_REDACT_KEY` or `--redact-key` before writing share artifacts.

## For contributors

See [CONTRIBUTING.md](CONTRIBUTING.md) for setup instructions and coding
conventions.

Useful local checks:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo xtask check-no-panic-family --mode blocking-allowlist
```

Policy and architecture references:

- [API_SURFACE.md](API_SURFACE.md)
- [ROADMAP.md](ROADMAP.md)
- [docs/CLIPPY_POLICY.md](docs/CLIPPY_POLICY.md)
- [docs/NO_PANIC_POLICY.md](docs/NO_PANIC_POLICY.md)
- [docs/FILE_POLICY.md](docs/FILE_POLICY.md)
- [docs/POLICY_ALLOWLISTS.md](docs/POLICY_ALLOWLISTS.md)

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your
option.
