# shiplog

> Review readiness with receipts.

## Install

```bash
cargo install shiplog --locked
```

## First useful loop

```bash
shiplog init --guided
shiplog doctor --setup
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog repair plan --latest
```

## What you get

- Setup preflight before intake.
- Status cockpit after each run.
- Evidence intake with receipts.
- Repair queue for missing evidence.
- Rerun/diff proof.
- Read-only share explanation before rendering.

## Read next

| Need | Doc |
| --- | --- |
| First run | [Rapid first-intake guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/rapid-first-intake.md) |
| Recurring loop | [Recurring review-loop guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/recurring-review-loop.md) |
| Setup | [Guided setup and doctor guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/guided-setup-doctor.md) |
| Repair | [Evidence repair loop guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/evidence-repair-loop.md) |
| Share | [Review-ready packet guide](https://github.com/EffortlessMetrics/shiplog/blob/main/docs/guides/review-ready-packet.md) |

## License

Dual licensed under [MIT](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-MIT) OR [Apache-2.0](https://github.com/EffortlessMetrics/shiplog/blob/main/LICENSE-APACHE), at your option.
