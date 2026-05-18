# Review-ready packet guide

Use this after a first intake has produced a packet and you want to know whether
it is ready to support a self-review conversation. The goal is not to generate
performance-review prose. The goal is to turn repaired evidence into a
defensible scaffold: readiness, claim candidates, missing-context prompts, and
share posture.

The loop is:

```text
setup -> collect -> repair -> rerun -> compare -> interpret -> share
```

If setup is new or uncertain, run the read-only front door before the first
packet:

```bash
shiplog init --guided
shiplog doctor --setup
shiplog sources status
shiplog doctor --setup --json
```

`doctor --setup` explains local setup state. `sources status` is the
source-only projection. `doctor --setup --json` is the same model for agents and
scripts. These commands do not collect evidence or render share artifacts.

For dogfood or review-cycle soak runs, use an explicit output directory and keep
using it for every follow-up command. This keeps artifacts out of unrelated
`./out` runs and makes `--latest` resolve the packet you are actively repairing.

```bash
OUT=./out/review-ready-soak
```

The examples below use the default `./out` for brevity. If you set `OUT`, add
`--out "$OUT"` to each command, or use the copy-ready commands printed by
`intake`, `review`, `repair diff`, and `runs diff`.

## Run the first packet

Start with setup readiness when needed, then intake, and read the report before
editing anything.

```bash
shiplog doctor --setup
shiplog sources status
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

Read these sections first:

- `Packet readiness`: whether the packet is ready, caveated, repairable, or
  still missing evidence.
- `Source Freshness` and `Skipped Sources`: what was included or unavailable.
- `Repair items`: safe local actions derived from report receipts.
- `Evidence debt`: curation or context gaps that can weaken the packet.

When repair items exist, the top-level `Next` handoff is intentionally
read-first: it should point at `repair plan` before write-producing commands.
Direct `journal add`, workstream split, or receipt-trimming commands may still
appear inside `Evidence debt` or `Top Fixups` as context, but use `repair plan`
to decide which actions are safe and receipt-backed.

If the packet has no evidence, do not treat that as failure. Treat it as the
first diagnostic run.

If the packet has source-backed evidence but still says `Needs curation` or
`Ready with caveats`, treat that as a usable but unfinished packet. Real
history often needs outcome context, receipt trimming, or workstream splitting
before the claim candidates are defensible.

## Repair locally

Print the repair queue:

```bash
shiplog repair plan --latest
```

Run the repair plan before copying individual fix commands from the packet. It
groups safe local actions first and keeps advisory items explicit.

For a missing manual-evidence repair item, add a local journal event from the
report-derived repair ID:

```bash
shiplog journal add --from-repair <repair_id>
```

If you used a non-default output directory, keep the same output directory on
the repair commands:

```bash
shiplog repair plan --out "$OUT" --latest
shiplog journal add --from-repair <repair_id> --out "$OUT" --latest
```

This writes the configured manual evidence file: `manual_events.yaml` for the
default scaffold, or the path from `[sources.manual].events` when your config
points somewhere else. It does not mutate GitHub, Jira, Linear, GitLab, or any
provider record. Replace generated placeholder context with what actually
happened before relying on the packet.

Some repair-plan entries intentionally have `no safe copyable command`. Those
are still useful: they name evidence debt, source posture, or share posture that
needs judgment instead of pretending Shiplog can fix it automatically.

## Rerun and compare

After the repair, rerun intake and compare both repair state and packet quality.

```bash
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog open packet --latest
```

With a non-default output directory:

```bash
shiplog intake --last-6-months --explain --out "$OUT"
shiplog repair diff --out "$OUT" --latest
shiplog runs diff --out "$OUT" --latest
shiplog open packet --out "$OUT" --latest
```

Read `repair diff` first:

- `Cleared`: a repair key was present in the older report and absent in the
  newer report.
- `New`: a repair key appears only in the newer report.
- `Still open`: the same repair remains.
- `Changed`: the repair key remains but the action, reason, or clear condition
  changed.

After a successful comparison, `repair diff` prints a `Next:` handoff to
`runs diff`, `open packet`, and `share explain manager`. Use that handoff when
you are following a non-default `--out` directory; it preserves the newer run ID
so the next commands stay on the repaired packet.

Then read `runs diff`:

- `Improved`: evidence counts, manual evidence, readiness, claim candidates, or
  cleared repairs moved in a better direction.
- `Changed`: a repair key stayed open but its reason, action, or clear
  condition changed. Treat this as neutral until you inspect the reason.
- `Regressed`: evidence, gaps, readiness, claim candidates, or repair state got
  worse.
- `Still weak`: skipped sources, open repairs, manual-only evidence, missing
  candidates, or other caveats still need human judgment.

Treat the `Next:` commands from `repair diff` and `runs diff` as the handoff.
They should preserve the selected `--out` directory and point at `open packet`
and `share explain` before any write-producing share command.

The useful outcome is not "no warnings." The useful outcome is knowing exactly
what improved and what still needs context.

## Interpret packet readiness

Open `packet.md` and start at `Packet Readiness`.

Common states:

- `Ready`: the packet has source-backed evidence and no open repair blockers.
- `Ready with caveats`: useful, but still has source gaps, manual-only evidence,
  evidence debt, or open caveats.
- `Needs repair`: fixable report-derived repair items remain.
- `Needs evidence`: there is not enough evidence to support claims yet.

Do not hide caveats. A caveated packet is often still useful if the packet says
what evidence supports it and what remains weak.

## Use claim candidates safely

Read `Claim Candidates` as scaffolding, not prose.

Each candidate should tell you:

- the claim title;
- evidence strength;
- supporting sources and receipts;
- caveats;
- missing-context prompts;
- safe share profiles.

Use candidates to decide what you can defend. Do not copy them as final review
paragraphs without adding your own specific context. Shiplog should not invent
impact, outcomes, or performance narratives.

When a candidate has `manual_only` evidence, answer the missing-context prompt:

```text
Which source-backed receipt could confirm this?
```

If there is no receipt, keep the claim narrow or leave it out.

## Explain share posture

Before rendering a manager or public profile, ask what the profile would include,
remove, and block.

```bash
shiplog share explain manager --latest
shiplog share explain public --latest
```

For a soak run:

```bash
shiplog share explain manager --out "$OUT" --latest
shiplog share explain public --out "$OUT" --latest
```

The command is read-only. It does not require `SHIPLOG_REDACT_KEY`, and it does
not write `profiles/<profile>/packet.md` or `share.manifest.json`. Use it to
answer:

- what the profile includes;
- what redaction removes;
- whether the missing redaction key blocks rendering;
- what still needs review before sharing.

The follow-up render commands are labeled `Render when ready:`. Treat them as
the path after you have reviewed the posture, not as work that `share explain`
already performed.

When you are ready to write a share profile:

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret
shiplog share verify manager --latest
shiplog share manager --latest --zip
```

On PowerShell, use the platform-specific redaction-key setup printed by
`share explain`:

```powershell
$env:SHIPLOG_REDACT_KEY='replace-with-a-stable-secret'
shiplog share verify manager --latest
shiplog share manager --latest --zip
```

For public sharing, use the strict public verification path:

```bash
shiplog share verify public --latest --strict
shiplog share public --latest --zip
```

Strict verification is a guardrail, not a privacy guarantee. Review the rendered
public packet before it leaves your organization.

Do not use the share commands as a reason to cut a release. In the post-0.8 soak
lane, the goal is to prove the packet is understandable and share-safe before
deciding whether the unreleased 0.9 candidate should ship.

## Stop conditions

Stop and use the packet when:

- readiness is `Ready` or `Ready with caveats`;
- the caveats are visible and acceptable for your audience;
- claim candidates are backed by receipts or explicitly marked as manual-only;
- missing-context prompts have been answered in your own words;
- share posture says the target profile is not blocked.

Keep repairing when:

- readiness is `Needs evidence`;
- a repair item has a safe local action you can complete;
- a skipped source would materially change the claims;
- a claim candidate depends only on memory and has no supporting receipt.

The defensible path is: collect, repair, rerun, compare, prepare, share.
