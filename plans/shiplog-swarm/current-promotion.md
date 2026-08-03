<!-- GENERATED FROM plans/shiplog-swarm/promotion-state.toml BY `cargo xtask promotion-state`. DO NOT EDIT BY HAND. -->
# Current shiplog-swarm Promotion

**Status:** completed; approved source governance follows the promotion
**Promoted swarm head:** `e83a0f4d96cfb944ba12e9601dd862723a68a4bc`
**Source promotion:** `EffortlessMetrics/shiplog#682`
**Source merge commit:** `2f69daf921d74fb9472584c2a0df31857cfa2384`
**Source governance:** `EffortlessMetrics/shiplog#656`
**Source governance:** `EffortlessMetrics/shiplog#679`
**Source governance:** `EffortlessMetrics/shiplog#681`
**Source governance:** `EffortlessMetrics/shiplog#684`
**Source governance:** `EffortlessMetrics/shiplog#686`
**Source post-merge proof:** `https://github.com/EffortlessMetrics/shiplog/actions/runs/30806322336`

## Included work

- `EffortlessMetrics/shiplog-swarm#248`
- `EffortlessMetrics/shiplog-swarm#251`
- `EffortlessMetrics/shiplog-swarm#253`
- `EffortlessMetrics/shiplog-swarm#254`
- `EffortlessMetrics/shiplog-swarm#255`
- `EffortlessMetrics/shiplog-swarm#256`
- `EffortlessMetrics/shiplog-swarm#252`
- `EffortlessMetrics/shiplog-swarm#247`
- `EffortlessMetrics/shiplog-swarm#249`
- `EffortlessMetrics/shiplog-swarm#250`
- `EffortlessMetrics/shiplog-swarm#257`
- `EffortlessMetrics/shiplog-swarm#258`
- `EffortlessMetrics/shiplog-swarm#259`
- `EffortlessMetrics/shiplog-swarm#260`
- `EffortlessMetrics/shiplog-swarm#261`
- `EffortlessMetrics/shiplog-swarm#262`
- `EffortlessMetrics/shiplog-swarm#269`
- `EffortlessMetrics/shiplog-swarm#265`
- `EffortlessMetrics/shiplog-swarm#271`
- `EffortlessMetrics/shiplog-swarm#272`
- `EffortlessMetrics/shiplog-swarm#274`
- `EffortlessMetrics/shiplog-swarm#275`
- `EffortlessMetrics/shiplog-swarm#277`
- `EffortlessMetrics/shiplog-swarm#268`
- `EffortlessMetrics/shiplog-swarm#282`
- `EffortlessMetrics/shiplog-swarm#288`
- `EffortlessMetrics/shiplog-swarm#289`
- `EffortlessMetrics/shiplog-swarm#290`
- `EffortlessMetrics/shiplog-swarm#295`
- `EffortlessMetrics/shiplog-swarm#299`
- `EffortlessMetrics/shiplog-swarm#293`
- `EffortlessMetrics/shiplog-swarm#291`
- `EffortlessMetrics/shiplog-swarm#292`
- `EffortlessMetrics/shiplog-swarm#296`
- `EffortlessMetrics/shiplog-swarm#301`
- `EffortlessMetrics/shiplog-swarm#303`
- `EffortlessMetrics/shiplog-swarm#305`
- `EffortlessMetrics/shiplog-swarm#307`
- `EffortlessMetrics/shiplog-swarm#308`
- `EffortlessMetrics/shiplog-swarm#310`
- `EffortlessMetrics/shiplog-swarm#311`
- `EffortlessMetrics/shiplog-swarm#312`
- `EffortlessMetrics/shiplog-swarm#313`
- `EffortlessMetrics/shiplog-swarm#314`
- `EffortlessMetrics/shiplog-swarm#315`
- `EffortlessMetrics/shiplog-swarm#316`
- `EffortlessMetrics/shiplog-swarm#317`
- `EffortlessMetrics/shiplog-swarm#318`
- `EffortlessMetrics/shiplog-swarm#332`
- `EffortlessMetrics/shiplog-swarm#333`
- `EffortlessMetrics/shiplog-swarm#334`
- `EffortlessMetrics/shiplog-swarm#335`
- `EffortlessMetrics/shiplog-swarm#337`

## Pending swarm work

- `EffortlessMetrics/shiplog-swarm#338`
- `EffortlessMetrics/shiplog-swarm#339`
- `EffortlessMetrics/shiplog-swarm#348`
- `EffortlessMetrics/shiplog-swarm#351`
- `EffortlessMetrics/shiplog-swarm#352`
- `EffortlessMetrics/shiplog-swarm#353`
- `EffortlessMetrics/shiplog-swarm#354`
- `EffortlessMetrics/shiplog-swarm#355`
- `EffortlessMetrics/shiplog-swarm#356`
- `EffortlessMetrics/shiplog-swarm#357`
- `EffortlessMetrics/shiplog-swarm#359`
- `EffortlessMetrics/shiplog-swarm#361`
- `EffortlessMetrics/shiplog-swarm#363`
- `EffortlessMetrics/shiplog-swarm#365`
- `EffortlessMetrics/shiplog-swarm#366`
- `EffortlessMetrics/shiplog-swarm#368`
- `EffortlessMetrics/shiplog-swarm#370`
- `EffortlessMetrics/shiplog-swarm#371`
- `EffortlessMetrics/shiplog-swarm#373`
- `EffortlessMetrics/shiplog-swarm#375`
- `EffortlessMetrics/shiplog-swarm#377`
- `EffortlessMetrics/shiplog-swarm#379`
- `EffortlessMetrics/shiplog-swarm#381`

## Source-authority decisions

- path: `.github/workflows/droid-review.yml`
  - reason: Canonical source review automation may publish reviews but cannot write product contents.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`
- path: `.github/workflows/droid-security-scan.yml`
  - reason: Canonical source scheduled security verification remains read-only while swarm may originate remediation.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`
- path: `.github/workflows/droid.yml`
  - reason: Canonical source review automation may publish comments but cannot mint identity or write product contents.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`
- path: `.github/workflows/release.yml`
  - reason: The source release writer remains authoritative while merged #391 carries the reviewed swarm candidate verification contract; this is a bounded rebind, not a transfer of release authority.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `f99dcbd1ec331c9665b16748b2493bc0a4f0c25f`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#391` at `ac9308542fc6ec4daa1337bf81303bcf443542e1`
  - consumed by: `(active)`
- path: `.github/workflows/security.yml`
  - reason: Canonical source scheduled security verification declares read-only permissions and the swarm remediation handoff.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`
- path: `.github/workflows/source-automation-guard.yml`
  - reason: Canonical source owns the merge-control guard and its synthetic proof dispatch after source PR #686.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`
- path: `policy/automation-authority.toml`
  - reason: Canonical source owns the source-role policy from #679 while swarm retains its distinct swarm-role policy.
  - source target: `175e8c8cee8110c4cf4a42d5534f5dffe45bf426`
  - swarm target: `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#351` at `5861dffe341d6f42cd145fe154c215c1c29836ca`
  - consumed by: `(active)`

## Truth hierarchy

1. Git refs and ancestry
2. GitHub PR / check state
3. `plans/shiplog-swarm/promotion-state.toml` (this promotion's source of truth)
4. Generated reports (`target/source-of-truth/*`, this file)
5. Historical archive (`plans/shiplog-swarm/implementation-plan.md`)

## Topology boundary

- Product development remains authoritative in `EffortlessMetrics/shiplog-swarm`.
- Source promotion uses a regular merge commit; do not squash.
- Release authority, tags, publishing, signing, and release workflows remain in `EffortlessMetrics/shiplog`.

## Next action

Prepare the next source promotion for the pending swarm range with `cargo xtask promote --swarm-sha $(git rev-parse swarm/main)`. Carry these receipts in the next substantive swarm PR; do not open a receipt-only refresh PR.
