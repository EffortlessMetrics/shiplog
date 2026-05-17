# Review-ready dogfood matrix

> **Audience:** post-0.8 release hold, product soak, and targeted test planning.
> **Status:** 0.9 remains paused. This matrix does not approve tag, publish, or
> GitHub release execution.
> **Companion docs:** [`docs/release/0.9.0-release-hold.md`](../release/0.9.0-release-hold.md),
> [`docs/release/0.9.0-readiness.md`](../release/0.9.0-readiness.md),
> [`docs/guides/review-ready-packet.md`](../guides/review-ready-packet.md).

This matrix turns the late post-0.8 dogfood receipts into a small set of flows
we either trust today or still need to exercise before deciding whether to
resume 0.9 release work.

It is a control-plane artifact, not a harness. The follow-up test PR should add
small targeted tests for gaps, not one giant end-to-end scenario.

## Release posture

`v0.8.0` is the latest shipped release. The review-ready packet work on `main`
is an unreleased 0.9 candidate, and release execution stays blocked while the
hold receipt exists.

Do not use this matrix as permission to run:

- `git tag v0.9.0`
- `git push origin v0.9.0`
- `cargo publish -p shiplog --locked`
- `gh release edit v0.9.0 --draft=false --latest`

## Core loop under soak

The loop under review is:

```bash
shiplog intake --last-6-months --explain
shiplog repair plan --latest
shiplog journal add --from-repair <repair_id>
shiplog intake --last-6-months --explain
shiplog repair diff --latest
shiplog runs diff --latest
shiplog open packet --latest
shiplog share explain manager --latest
shiplog share verify manager --latest
shiplog share manager --latest
```

The product claim is narrow:

- read-first handoffs before write-producing repairs or share rendering;
- source repair items clear only when matching evidence appears;
- packet quality stays receipt-derived;
- `share explain` remains read-only;
- manager/public rendering still fails closed without required redaction setup.

## Matrix

| Row | Flow | Current trust signal | Still needs exercise |
| --- | --- | --- | --- |
| 1 | Empty directory, no tokens | Intake can produce an honest first diagnostic packet; zero-event sources are not listed as `Good`; repairable flows point at `repair plan` first (#380, #381, #382). | Periodic clean-directory transcript should confirm the top-level `Next` block remains read-first and does not advertise write-first commands. |
| 2 | Local git plus empty valid manual journal | Empty manual evidence is visible without pretending the packet is complete; repair handoffs can route through `journal add --from-repair` when the journal is valid (#338, #344, #381). | Targeted test should keep the first safe action on `repair plan`, then `journal add --from-repair`, not direct `journal add --date`. |
| 3 | Local git plus malformed manual journal | Malformed manual journals block copyable journal repair handoffs and route the user to setup repair instead (#377). | Keep a narrow regression test so future wording or repair-plan edits cannot reintroduce unsafe copyable journal commands. |
| 4 | Repaired manual-only packet | Manual repair can improve the packet, rerun intake, and hand off to `repair diff` before planning again (#343, #344, #388). | Confirm `runs diff` shows quality movement without overstating source-backed confidence for manual-only evidence. |
| 5 | Skipped provider sources | Autodetected skipped GitHub/GitLab/Jira/Linear/JSON sources stay visible across coverage, packet, review output, and `share explain` (#386). Repair diff requires matching source evidence before clearing provider source repairs (#387). | Add targeted proof that a journal-only rerun cannot clear provider repair items and that skipped-source caveats remain visible in share posture. |
| 6 | Old report without `packet_quality` | Compatible old reports remain valid; report-facing commands can explain that richer packet-quality signals require rerunning intake (#365). | Harden `runs diff`, `share explain`, and packet-opening paths against reports missing `packet_quality`, `claim_candidates`, `share_posture`, or `repair_items`. |
| 7 | Manager `share explain` without key | `share explain manager` is read-only, surfaces packet readiness and evidence debt, and labels follow-up writes as `Render when ready` (#347, #378, #389). | Pin no-write behavior so explanation cannot create profile artifacts after future edits. |
| 8 | Public share explain/verify path | Public sharing stays separated into read-only explanation and fail-closed verification/rendering; strict public verification remains the safer pre-render path (#315, #347, #389). | Exercise public `share explain` with packet debt and strict-review caveats, then verify public rendering still requires the expected redaction/profile guardrails. |

## Targeted test backlog

The next test PR should prefer small assertions around these gaps:

- public `share explain` surfaces strict-review caveats together with packet
  debt;
- repaired rerun `Next` chooses `repair diff` before `repair plan`;
- old reports degrade gracefully in `share` and `runs` commands;
- no-write surfaces remain no-write after future edits;
- repair diff cannot clear provider repair items without provider evidence.

## Release decision use

Use this matrix to decide whether the review-ready loop has been dogfooded
enough to ask for a release decision. It is not itself the release decision.

The matrix is green enough to consider resuming 0.9 only when:

- every row has either targeted test proof or an explicit accepted caveat;
- no P0/P1 UX bug remains in the core review-ready loop;
- release-hold receipts are current;
- `v0.9.0` still has not been tagged, published to crates.io, or released on
  GitHub;
- the owner explicitly approves release execution.
