<h1 align="center">shiplog</h1>

> **Development repository:** Contributions and normal development happen in
> [`EffortlessMetrics/shiplog-swarm`](https://github.com/EffortlessMetrics/shiplog-swarm).
> The `EffortlessMetrics/shiplog` repository is the public release source; new
> contributors do not need its maintainer-only promotion setup.

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
  <em>Review readiness with receipts: capture evidence, refresh it, and share safely.</em>
</p>

> **Development repository.** Contributors open pull requests in
> [`EffortlessMetrics/shiplog-swarm`](https://github.com/EffortlessMetrics/shiplog-swarm).
> End users should install releases and report release-facing issues through
> [`EffortlessMetrics/shiplog`](https://github.com/EffortlessMetrics/shiplog).

shiplog turns work evidence into a review-readiness loop: diagnose setup,
inspect status, collect receipts, repair gaps, rerun, compare, and share
safely.

Current shipped release: `v0.11.0`.

## The problem

Performance reviews ask what shipped, what mattered, and what evidence supports
it. Most people discover missing evidence too late.

shiplog keeps the evidence loop receipt-backed:

```text
capture -> update -> packet -> curate -> share safely
```

It is for individual contributors, tech leads, and anyone who wants a
repeatable evidence trail for self-reviews, promo packets, or brag documents.

## What works in 0.11

| Surface | Status | Command |
|---------|--------|---------|
| First packet | Ready | `shiplog intake` |
| Home screen | Ready | `shiplog` |
| Quick evidence capture | Ready | `shiplog add "what changed"` |
| Evidence refresh | Ready | `shiplog update` |
| Safe next action | Ready | `shiplog next` |
| Current packet | Ready | `shiplog open` |
| Manager/public share | Ready | `shiplog share manager` / `shiplog share public` |
| Objective setup readiness | Ready | `shiplog doctor --setup --for intake` |
| Agent review state | Ready | `shiplog status --latest --json` |
| Reminder/CI gate | Ready | `shiplog status --check` |
| Advanced GitHub harvest | Ready | `shiplog github activity plan` |

## Quick start

From a work directory, run the one command that creates the first packet:

```bash
shiplog intake
```

Intake uses the default six-month window, discovers usable local evidence,
creates `shiplog.toml` and `manual_events.yaml` when needed, and records
missing optional providers without requiring credentials. Add `--explain` when
you want per-source decisions in the terminal.

Open the packet after intake when you want the rendered artifact immediately:

```bash
shiplog open
```

## Normal workflow

Once the first packet exists, ordinary use is short:

```bash
shiplog add "Resolved the customer import retry incident" \
  --impact "Protected the next import window"
shiplog update
shiplog open
shiplog
shiplog share manager
```

`shiplog` is read-only and shows packet age, source caveats, evidence gaps,
and one receipt-derived next action. `shiplog update` is the explicit write
command for refreshing evidence and rebuilding the packet. Sharing remains an
explicit command and explains and verifies its profile before rendering.

## Install

Install from crates.io:

```bash
cargo install shiplog --locked
```

Build from a source checkout:

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

## Setup troubleshooting

Setup commands are optional for the first packet. Use them when you want to
inspect or repair configuration before collecting evidence:

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

Use `shiplog sources list` to see which sources are configured and enabled, and
`shiplog sources enable --source <name>` / `shiplog sources disable --source <name>`
to toggle a source on or off. The toggle flips only the `enabled` flag in
`shiplog.toml`, keeps your comments and provider records intact, and never
writes tokens.

## First packet

Collect usable evidence from the default six-month window:

```bash
shiplog intake
shiplog open
```

`intake` writes run artifacts under `out/<run_id>/`, including
`packet.md`, `intake.report.md`, `intake.report.json`,
`ledger.events.jsonl`, `coverage.manifest.json`, and a bundle manifest.
`status --latest` reads those receipts and tells you whether the next safe
step is repair, rerun, diff, or share explanation.

For a quick human note that automation cannot infer, use:

```bash
shiplog add "Led the rollback review" --impact "Reduced recovery risk"
shiplog update
```

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
shiplog status --check
```

For a read-only home screen, run `shiplog`. For a read-only typed next-action
projection, run `shiplog next` or `shiplog next --json`. For cron or CI, use
`shiplog status --check`; it does not contact providers or write evidence.

`status --check` is a cron/CI gate: it prints the usual status (add `--json`
for the model) and exits `0` when the loop is ready or `1` when it needs
action, so a scheduled job can alert only when there is work to do.

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
| 0.11 release readiness | [docs/release/0.11.0-readiness.md](docs/release/0.11.0-readiness.md) |
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

Deterministic HMAC-SHA256 redaction aliases are available for manager and public
share profiles. Rendering those profiles is explicit and fail-closed: provide
`SHIPLOG_REDACT_KEY` or `--redact-key` before writing share artifacts.

## For contributors

Clone `shiplog-swarm` and run the same small gate required by pull requests:

```bash
git clone https://github.com/EffortlessMetrics/shiplog-swarm.git
cd shiplog-swarm
cargo build --workspace --locked
cargo xtask ci-small
```

After the public checkout is acquired, the build and gate require no retained
checkout credential, provider token, GitHub CLI installation, release remote,
hook, or generated file. See [CONTRIBUTING.md](CONTRIBUTING.md) for the smallest
PR loop and coding conventions.

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
