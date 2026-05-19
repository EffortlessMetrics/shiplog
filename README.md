# shiplog

<p align="center">
  <a href="https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/EffortlessMetrics/shiplog/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="https://codecov.io/gh/EffortlessMetrics/shiplog"><img alt="Codecov" src="https://codecov.io/gh/EffortlessMetrics/shiplog/branch/main/graph/badge.svg" /></a>
  <a href="https://crates.io/crates/shiplog"><img alt="crates.io" src="https://img.shields.io/crates/v/shiplog.svg" /></a>
  <a href="https://docs.rs/shiplog"><img alt="docs.rs" src="https://docs.rs/shiplog/badge.svg" /></a>
  <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg" /></a>
</p>

> Review readiness with receipts: setup, status, intake, repair, rerun, diff, and share safely.

## The problem

Performance reviews ask what shipped, what mattered, and what evidence supports it.
Most people discover missing evidence too late.

shiplog keeps the loop visible before the deadline:

```text
setup -> status -> intake -> repair -> rerun -> diff -> share explain
```

## What works in 0.9

| Surface | Command | What it answers |
| --- | --- | --- |
| Setup preflight | `shiplog doctor --setup` | Is the runway clear? |
| Agent setup state | `shiplog doctor --setup --json` | Can automation proceed safely? |
| Review cockpit | `shiplog status --latest` | Where am I in the loop? |
| Agent review state | `shiplog status --latest --json` | What is the next safe action? |
| Evidence intake | `shiplog intake --last-6-months --explain` | What evidence was collected? |
| Repair queue | `shiplog repair plan --latest` | What can be safely fixed? |
| Local repair | `shiplog journal add --from-repair <repair_id>` | Add missing manual evidence. |
| Repair movement | `shiplog repair diff --latest` | What repair keys cleared? |
| Packet movement | `shiplog runs diff --latest` | Did readiness improve? |
| Share posture | `shiplog share explain manager --latest` | What is safe before rendering? |

0.9 remains a held candidate on `main`; crates.io currently publishes 0.8.0.
Treat release docs as the source of truth for readiness and hold posture.

## Install

```bash
cargo install shiplog --locked
```

From a source checkout (0.9 candidate):

```bash
git clone https://github.com/EffortlessMetrics/shiplog.git
cd shiplog
cargo install --path apps/shiplog --locked
```

## First useful loop

```bash
shiplog init --guided
shiplog doctor --setup
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
```

## Repair and compare

```bash
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
```

## Share safely

```bash
shiplog share explain manager --latest
shiplog share verify manager --latest
```

## What shiplog does not do

- It does not write your review prose.
- It does not score employees.
- It does not mutate provider records.
- It does not render manager/public packets from `status`.
- It does not query providers from `doctor` or `status`.
- It does not treat missing setup as weak evidence.

## Documentation

| Need | Start here |
| --- | --- |
| First run | [docs/guides/rapid-first-intake.md](docs/guides/rapid-first-intake.md) |
| Setup | [docs/guides/guided-setup-doctor.md](docs/guides/guided-setup-doctor.md) |
| Recurring use | [docs/guides/recurring-review-loop.md](docs/guides/recurring-review-loop.md) |
| Evidence repair | [docs/guides/evidence-repair-loop.md](docs/guides/evidence-repair-loop.md) |
| Packet interpretation | [docs/guides/review-ready-packet.md](docs/guides/review-ready-packet.md) |
| Config | [docs/config-reference.md](docs/config-reference.md) |
| Schemas | [docs/schemas/](docs/schemas) |
| Release status | [docs/release/0.9.0-readiness.md](docs/release/0.9.0-readiness.md) |

## License

Dual licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE), at your option.
