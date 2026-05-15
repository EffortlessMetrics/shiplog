# Rapid first-intake guide

Use this guide the **first time** you run shiplog and need a defensible
review pack from a literal empty directory. The product contract this
guide is the how-to companion for lives at
[`docs/product/rapid-first-intake.md`](../product/rapid-first-intake.md);
this file shows the commands and what to read in the output, not the
design rationale.

The promise:

```text
useful immediately, auditable later
```

One command produces a packet you can hand to a reviewer. The first
read of the output tells you what worked, what was skipped, what is
fresh, and what to add next.

## Who this is for

You are:

- preparing a self-review, promo packet, or "what did you ship?" doc
  with a deadline,
- working from an **empty directory** with no prior `shiplog.toml`, no
  `manual_events.yaml`, no `out/` folder,
- willing to run one command, read one report, and curate later if the
  first pack is good enough.

If you are running a weekly review cycle on an established setup, use
[`docs/guides/review-cycle.md`](review-cycle.md). If you already have a
setup but the form is due tonight, use
[`docs/guides/review-deadline.md`](review-deadline.md).

## Setup

You need either the release binary or a Rust toolchain. Either is
fine — pick one.

```bash
# Option A: release binary (no Rust required).
# See docs/install.md for per-platform downloads + checksums.
shiplog --version

# Option B: Cargo install (requires Rust 1.95+).
cargo install shiplog --locked
shiplog --version
```

Provider tokens are **optional**. Set the ones you have; shiplog skips
the rest and records why.

```bash
export GITHUB_TOKEN=...   # GitHub source
export GITLAB_TOKEN=...   # GitLab source
export JIRA_TOKEN=...     # Jira source
export LINEAR_API_KEY=... # Linear source
```

If you have **zero** tokens, the first run still succeeds. shiplog
scaffolds a starter `manual_events.yaml` and treats your own typed
notes as the one working source — see
[Add manual evidence](#add-manual-evidence) below.

## One-command cold-start

From an empty directory:

```bash
shiplog intake --last-6-months --explain
shiplog open intake-report --latest
shiplog open packet --latest
```

That is the whole happy path. `intake` does the work, then `open`
launches the durable report and the rendered pack in your platform's
default markdown viewer. `--explain` prints per-source decisions and
repair hints to the terminal so you can see what happened without
reading the report first.

The first run creates the following under `./out/<run_id>/`:

| File                          | What it carries                                                                                 |
|-------------------------------|--------------------------------------------------------------------------------------------------|
| `packet.md`                   | The rendered review pack: executive summary, per-workstream evidence, claim prompts.            |
| `intake.report.md`            | Durable checklist: readiness, freshness, included/skipped sources, evidence debt, next commands. |
| `intake.report.json`          | Machine-readable form of the above (schema: `docs/schemas/intake-report-v1.md`).                 |
| `ledger.events.jsonl`         | Canonical event ledger — every claim in the packet traces here.                                  |
| `coverage.manifest.json`      | API query windows, pagination hits, and recorded gaps.                                           |
| `workstreams.suggested.yaml`  | Auto-clustered workstream proposal (regenerable; safe to edit into `workstreams.yaml`).          |
| `bundle.manifest.json`        | SHA256 checksum per file for integrity verification.                                             |

`shiplog.toml` and `manual_events.yaml` are scaffolded next to your
current directory if they did not exist. They are yours to edit;
shiplog will not overwrite them.

## How to read `intake.report.md`

Open the report first — it is the reviewer's view of "what is in this
pack and what's missing." Read the sections in this order:

1. **Header** — run id, packet readiness, window, config path, packet path.
2. **Redaction profile** — one line saying which profile rendered this run.
   On a first run this is always `internal` (no redaction).
3. **Where to Look** — pointers to the other artifacts (event ledger,
   coverage, freshness, full artifact list).
4. **Source Freshness** — per-source state. See
   [What "Source Freshness" means](#what-source-freshness-means).
5. **Included Sources** — sources that contributed events, with counts.
6. **Skipped Sources** — sources that did not run, with reasons.
7. **Source Decisions** — every configured source's decision (included or
   skipped) with a repair hint where applicable.
8. **Repair Sources** — copy-ready commands grouped by source for the
   ones you can fix today.
9. **Evidence Debt** — quality gaps the run detected, severity-ranked.
10. **Good / Needs Attention** — the readiness summary you'd lead with
    when talking to a reviewer.
11. **Top Fixups / Journal Suggestions** — the highest-value curation
    actions, with copy-ready commands.
12. **Share Commands** — exact commands to render manager and public
    profiles.
13. **Next Commands** — what to run next (rerun, curate, share).
14. **Artifacts** — every file path the run produced.

You do not have to read every section. **Header → Source Freshness →
Skipped Sources → Needs Attention** is enough to decide whether the
pack is shareable now.

## How to read `packet.md`

`packet.md` is the artifact you give to a reviewer. It opens with an
executive summary derived from the workstreams + event counts, then
per-workstream sections with anchor receipts and claim prompts. You
write the narrative; shiplog provides the evidence.

If the executive summary looks thin or the per-workstream sections are
mostly empty, that is a signal to either curate workstreams or add
manual evidence — not a signal that shiplog failed.

## What "Source Freshness" means

The `## Source Freshness` section answers **"is the evidence in this
pack fresh, cached, or missing?"** without making the reviewer read
the cache directly. One entry per source. Status values you will see
on a first run:

- **`fresh`** — the source ran this run and returned current data
  (live API fetch, or read from current local input). On a cold-start
  run, every source that succeeded is `fresh` because the cache was
  empty.
- **`cached`** — the source served all its data from a valid cache
  entry, no live fetch happened. Only seen on a warm rerun after a
  prior successful run.
- **`stale`** — the source served at least one expired cache entry
  through a proven `CacheLookup::Stale` receipt.
- **`skipped`** — the source was intentionally not attempted (no
  token, configured `enabled = false`, missing local path).
- **`unavailable`** — the source was attempted but produced no usable
  result (transient error, partial fetch).

Each entry also carries `cache: N hit / M miss` when the source uses a
cache, and a free-form reason for `stale`, `skipped`, and `unavailable`.
The machine-readable form is in `intake.report.json` under
`source_freshness` for tools that consume the receipt directly. Shiplog
must not infer `stale` from a miss; it can only come from a cache lookup
that proves an expired row existed.

## What "Needs evidence" means

`intake.report.json`'s `readiness` field is one of:

```text
Ready for review   — at least one source produced events, evidence debt is low
Needs curation     — coverage is fine, but workstreams or receipts need a pass
Needs evidence     — zero events collected, or every source was skipped
Needs repair       — sources were attempted but errored; rerun after fixing
```

`Needs evidence` is the typical state of a literal empty-directory
first run: the scaffolded `manual_events.yaml` is empty, every
provider source was skipped, the manual source succeeded with zero
events. The run is honest, not broken. Add evidence and rerun.

## Add manual evidence

The fastest way to add evidence between intake runs is `shiplog
journal add`. It writes to `manual_events.yaml` with a normalised
shape — no hand-editing under deadline pressure.

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Debugged customer import incident" \
  --workstream "Customer Reliability" \
  --impact "Identified the upstream export shape before the next import window"

shiplog journal list
```

Attach a receipt when you have one:

```bash
shiplog journal add \
  --date 2026-05-08 \
  --title "Validated rollback checklist" \
  --workstream "Platform Reliability" \
  --receipt doc=https://example.invalid/rollback-checklist
```

Manual events are first-class evidence — the rendered packet treats
them the same as a PR or a ticket. Keep them factual.

## How to rerun

Run `intake` again. The new run writes a new `<run_id>` directory; the
prior `out/<run_id>/` stays intact for comparison.

```bash
shiplog intake --last-6-months --explain
```

What is preserved across reruns:

- `manual_events.yaml` is never overwritten by intake.
- `shiplog.toml` is never overwritten by intake.
- A user-curated `workstreams.yaml` (if you promoted one from the
  `suggested` file) is reused by the new run.
- The cache directory is reused; a warm rerun will show `cached` in
  `## Source Freshness` for any source whose data is still valid in
  the cache.

What changes:

- `<run_id>` is a fresh sortable timestamp + hash.
- `workstreams.suggested.yaml` is regenerated.
- All artifacts under the new run directory are fresh.

## Share safely

Manager and public profiles are deterministic: same key + same input =
same aliases across runs. They fail closed without
`SHIPLOG_REDACT_KEY`.

```bash
export SHIPLOG_REDACT_KEY=replace-with-a-stable-secret

shiplog share verify manager --latest
shiplog share manager --latest --zip

shiplog share verify public --latest --strict
shiplog share public --latest --zip
```

`share verify` is read-only — use it before writing a share packet to
confirm coverage, gaps, and key state. `--strict` on the public
profile scans the rendered output for obvious raw URLs and original
names. Redaction reduces accidental disclosure; it is not a substitute
for thinking about whether a specific receipt should be shared.

## Repair a skipped source

When the report's `Repair sources` section lists a fix, run the
commands it printed and then rerun intake. The hints do not include
secret values.

```text
GitHub:
  export GITHUB_TOKEN=...
  shiplog intake --last-6-months --explain

Jira:
  export JIRA_TOKEN=...
  shiplog identify jira --auth-user you@example.com
  shiplog intake --last-6-months --explain
```

Fix one source at a time, then rerun. The rerun creates a new run
directory and preserves your curation and manual events.

## No-network rehearsal

You can rehearse the cold-start path against checked-in fixtures
without any provider tokens:

```bash
scripts/demo-review-rescue.sh --out ./out/rapid-first-intake-demo
```

On Windows PowerShell:

```powershell
pwsh -File .\scripts\demo-review-rescue.ps1 -Out .\out\rapid-first-intake-demo
```

The demo exercises local git, JSON, and manual fixture inputs with the
same intake → report → review → share verify shape a real run uses.

## See also

- [`docs/product/rapid-first-intake.md`](../product/rapid-first-intake.md) — the
  product contract this guide is the how-to companion for.
- [`docs/guides/review-deadline.md`](review-deadline.md) — deadline-pressure
  flow for users with an established setup.
- [`docs/guides/review-cycle.md`](review-cycle.md) — repeat review cycles.
- [`docs/guides/evidence-repair-loop.md`](evidence-repair-loop.md) — turn a
  rough first packet into a better rerun packet.
- [`docs/config-reference.md`](../config-reference.md) — full `shiplog.toml`
  field reference.
- [`docs/schemas/intake-report-v1.md`](../schemas/intake-report-v1.md) — the
  intake report JSON contract (including `source_freshness`).
- [`README.md`](../../README.md) — top-level quick start and feature list.
