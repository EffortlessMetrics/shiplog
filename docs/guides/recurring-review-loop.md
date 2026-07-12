# Recurring review-loop guide

Start here if you want to use Shiplog weekly, monthly, or for manager/promotion
packet prep. First use is intentionally shorter: run `shiplog intake`, then
use the normal `add` and `update` workflow to keep the receipt-backed packet
current. The detailed sections below remain the receipt-backed recurring loop
for diagnosis and audit.

The goal is not to generate a report once. The goal is to keep review readiness
visible:

```text
add -> update -> open -> next -> share explicitly
```

`doctor --setup` remains the setup preflight. `status --latest` is the review
loop cockpit after setup exists.

## Start with status

Run status before deciding what to do next:

```bash
shiplog status --latest
```

Use JSON when an agent or script needs the same state without scraping terminal
prose:

```bash
shiplog status --latest --json
```

Read the sections in this order:

1. `Setup`: whether local setup is usable.
2. `Latest run`: which run/report status read.
3. `Evidence`: packet readiness, included sources, unavailable sources.
4. `Repair`: open items, safe writes, setup-blocked writes.
5. `Diff`: whether comparable run receipts exist.
6. `Share`: manager/public posture.
7. `Next`: the safe next command, with `[read-only]` or `[writes]`.
8. `Receipts`: the files or setup model supporting the status.

Status is read-only. It should not query providers, mutate config, run intake,
repair evidence, render share artifacts, scrape `packet.md`, or generate review
prose.

## Normal monthly loop

Use this when you are preparing a manager packet or keeping a review cycle warm.

```bash
shiplog add "Resolved the customer import retry incident" \
  --impact "Protected the next import window"
shiplog update
shiplog open
shiplog
```

`shiplog update` collects current evidence, compares it with the prior run,
rebuilds the packet, and prints the next useful action. It does not add manual
evidence, apply repairs, or render a share profile. Add context first while it
is fresh; use the diagnostic loop below when the home screen reports a gap.

The important rule is:

```text
status first, then the next receipt-producing command.
```

If status says `ready_to_collect`, run intake or update. If it says `needs_repair`, run
`repair plan` before copying write commands. If it says `repair_in_progress`,
rerun intake before asking for diff. If it says `share_blocked`, do not render a
manager or public packet.

## Weekly self-review

Weekly use can be lighter. You are looking for drift, not finishing a packet.

```bash
shiplog add "Led rollback review" --impact "Reduced recovery risk"
shiplog update
```

For a detailed weekly diagnostic pass:

```bash
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog repair plan --latest
```

Stop after `repair plan` when the action is a reminder rather than urgent
evidence work. Add manual evidence while the context is fresh:

```bash
shiplog journal add \
  --date 2026-05-18 \
  --title "Resolved import retry failure" \
  --workstream "Customer Reliability" \
  --impact "Kept the next import window on schedule"
```

Then rerun status:

```bash
shiplog status --latest
```

If status still points to intake, rerun intake when you want the packet to
reflect the new manual evidence.

## Monthly manager prep

For manager prep, use status to avoid sharing too early.

```bash
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
shiplog repair plan --latest
shiplog repair diff --latest
shiplog runs diff --latest
shiplog share explain manager --latest
shiplog share verify manager --latest
```

Render only after the share commands say prerequisites are satisfied:

```bash
shiplog share manager --latest
```

`share explain` is read-only. `share verify` fails closed. `share manager`
writes profile artifacts.

## Promotion packet prep

Promotion packets need stronger receipts and more human context. Use the normal
loop, then inspect claim candidates and missing-context prompts before sharing.

```bash
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog open packet --latest
shiplog share explain manager --latest
```

Do not treat claim candidates as final review prose. They are evidence
scaffolds. Add specific human context for impact, scope, and who benefited
before using them in a promotion packet.

## Local-only mode

Use this when provider tokens are unavailable or you want a private local
routine.

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog status --latest
shiplog intake --last-6-months --explain
shiplog status --latest
```

Expected posture:

- manual source is ready after guided init;
- local Git is ready when the current directory is a readable Git repository;
- token-backed providers are disabled or unavailable, not weak evidence;
- share remains blocked until `SHIPLOG_REDACT_KEY` is configured.

Missing provider tokens do not prevent a local-only loop. They should remain
visible as setup/source caveats.

## Token-backed GitHub mode

Use setup commands before intake so missing credentials are not discovered late.

```bash
shiplog doctor --setup
shiplog sources status
shiplog status --latest
```

If GitHub is enabled but unavailable:

```bash
export GITHUB_TOKEN=...
shiplog sources status
```

Then collect:

```bash
shiplog intake --last-6-months --explain
shiplog status --latest
```

Status should summarize collected GitHub evidence only after intake has receipts.
Token presence alone is setup readiness, not evidence quality.

For a multi-year actor harvest with owner filters, budget receipts, and resume,
use the [GitHub activity harvest guide](github-activity-harvest.md) instead of
hand-running year slices.

## Public-share cautious mode

Public packets are strict. Use status and share explanation before rendering.

```bash
shiplog status --latest
shiplog share explain public --latest
shiplog share verify public --latest --strict
```

Render only after verification:

```bash
shiplog share public --latest
```

If status or share explain reports missing redaction setup, do not render. Set a
stable redaction key first:

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
```

Public redaction does not decide whether a receipt is appropriate to share. It
only applies the public profile policy.

## Agent-assisted mode

Agents should use JSON surfaces, not terminal prose scraping.

```bash
shiplog doctor --setup --json
shiplog status --latest --json
```

Safe agent decisions:

| Status state | Agent action |
|--------------|--------------|
| `needs_setup` or `blocked` | Run or show `doctor --setup` / `sources status`; do not run evidence repair writes. |
| `ready_to_collect` | Run intake when the user wants a fresh packet. |
| `needs_evidence` or `needs_repair` | Run `repair plan` before any write-producing repair command. |
| `repair_in_progress` | Rerun intake, then run `repair diff` / `runs diff`. |
| `ready_with_caveats` | Explain caveats before share verification. |
| `share_blocked` | Do not render manager/public packets. |
| `ready_to_share` | Use explicit share commands; status itself still does not render. |

Every next action includes:

- `command`;
- `reason`;
- `preconditions`;
- `priority`;
- `writes`.

Treat `writes = true` as a stop-and-confirm point unless the user already asked
for that write.

## Non-default output directories

For recurring runs, it is often cleaner to keep a named output directory:

```bash
OUT=./out/review-loop
shiplog status --out "$OUT" --latest
shiplog intake --out "$OUT" --last-6-months --explain
shiplog status --out "$OUT" --latest
shiplog repair plan --out "$OUT" --latest
shiplog repair diff --out "$OUT" --latest
shiplog runs diff --out "$OUT" --latest
shiplog share explain manager --out "$OUT" --latest
```

Use the copy-ready commands printed by status, intake, repair diff, and runs
diff when they include an explicit `--out`; that keeps the loop on the same run
family.

## What status should refuse to do

Status should not:

- call providers;
- run intake implicitly;
- apply repair commands;
- render manager or public profile packets;
- scrape `packet.md` as machine truth;
- generate performance-review prose;
- hide skipped sources or share blockers.

Status answers:

```text
Where am I?
What is blocking me?
What is safe next?
Which receipt proves that?
```

Everything else belongs in the underlying command.
