# Evidence repair loop guide

Use this after `shiplog intake` produces a rough packet and the report says what
is missing. The loop is intentionally local and receipt-derived: repair commands
read `intake.report.json`, journal repair writes `manual_events.yaml`, and no
provider tickets or source records are edited.

## Start from a report

Run intake and open the report first.

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
```

Read the header, `Source Freshness`, `Skipped Sources`, and `Needs Attention`.
If the packet is thin because no events were collected, the report should include
a manual evidence repair item.

## Print the repair queue

```bash
shiplog repair plan --latest
```

Each item has a stable `repair_id`, a `repair_key`, a reason, receipts, and a
safe action. For missing manual evidence, the command is:

```bash
shiplog journal add --from-repair <repair_id>
```

Use `--out <dir>` when the run directory is not under `./out`.

```bash
shiplog repair plan --out ./out/review --latest
shiplog journal add --from-repair <repair_id> --out ./out/review --latest
```

`journal add --from-repair` resolves the latest report, verifies that the repair
item is a `journal_add` action, adds tags that preserve the repair ID, and fills
safe default title/description/date values from the report. You can still add
specific receipts, workstream, or impact text when you have them.

## Rerun and compare

After adding evidence, rerun intake.

```bash
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
```

Read the diff groups this way:

| Group | Meaning |
|-------|---------|
| `Cleared` | A repair key existed in the older report and is absent from the newer report. |
| `New` | A repair key appears only in the newer report. |
| `Still open` | The same repair key is still present with the same reason/action/clear condition. |
| `Changed` | The same repair key is present but the reason, action, or clear condition changed. |

Then open the packet again.

```bash
shiplog open packet --latest
```

The useful outcome is not "all warnings disappeared." The useful outcome is that
the next packet contains more evidence, the cleared repair keys are visible,
`runs diff` shows packet-quality movement, and any remaining repair items still
point back to report receipts.

## Safety checks

- `repair plan` and `repair diff` read intake reports; they do not rediscover
  live source state.
- `journal add --from-repair` acts only on `journal_add` repair items.
- Repair commands must not include token values or secret material.
- The loop should not inspect `intake.report.md` for machine behavior; use
  `intake.report.json`.
- Provider setup fixes are manual operator actions, followed by another intake
  run. Shiplog does not mutate provider records in this loop.

## See also

- [`rapid-first-intake.md`](rapid-first-intake.md) for the first run from an
  empty directory.
- [`review-ready-packet.md`](review-ready-packet.md) for interpreting readiness,
  claim candidates, missing context, and share posture after repair.
- [`review-cycle.md`](review-cycle.md) for repeat review workflows.
- [`../schemas/intake-report-v1.md`](../schemas/intake-report-v1.md) for the
  report fields that repair commands consume.
