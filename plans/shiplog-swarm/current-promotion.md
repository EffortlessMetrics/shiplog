<!-- GENERATED FROM plans/shiplog-swarm/promotion-state.toml BY `cargo xtask promotion-state`. DO NOT EDIT BY HAND. -->
# Current shiplog-swarm Promotion

**Status:** completed; approved source governance follows the promotion
**Promoted swarm head:** `c4fdba223d1c5c5b99a95b159ab8123d83d4b842`
**Source promotion:** `EffortlessMetrics/shiplog#655`
**Source merge commit:** `160d430f1a5af338537e35ff98b8ddda14d4673c`
**Source governance:** `EffortlessMetrics/shiplog#656`
**Source governance:** `EffortlessMetrics/shiplog#679`
**Source governance:** `EffortlessMetrics/shiplog#681`

## Included work

- `EffortlessMetrics/shiplog-swarm#238`

## Pending swarm work

- `EffortlessMetrics/shiplog-swarm#247`
- `EffortlessMetrics/shiplog-swarm#248`
- `EffortlessMetrics/shiplog-swarm#249`
- `EffortlessMetrics/shiplog-swarm#250`
- `EffortlessMetrics/shiplog-swarm#251`
- `EffortlessMetrics/shiplog-swarm#252`
- `EffortlessMetrics/shiplog-swarm#253`
- `EffortlessMetrics/shiplog-swarm#254`
- `EffortlessMetrics/shiplog-swarm#255`
- `EffortlessMetrics/shiplog-swarm#256`
- `EffortlessMetrics/shiplog-swarm#257`
- `EffortlessMetrics/shiplog-swarm#258`
- `EffortlessMetrics/shiplog-swarm#259`
- `EffortlessMetrics/shiplog-swarm#260`
- `EffortlessMetrics/shiplog-swarm#261`

## Source-authority decisions

- path: `.github/workflows/droid-review.yml`
  - reason: The canonical source repository retains authority for the release and security automation surface during this bounded promotion.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
  - consumed by: `(active)`
- path: `.github/workflows/droid-security-scan.yml`
  - reason: The canonical source repository retains authority for the release and security automation surface during this bounded promotion.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
  - consumed by: `(active)`
- path: `.github/workflows/release.yml`
  - reason: The canonical source repository retains authority for the release and security automation surface during this bounded promotion.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
  - consumed by: `(active)`
- path: `.github/workflows/droid.yml`
  - reason: The canonical source review automation may publish comments but cannot mint identity or write product contents.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
  - consumed by: `(active)`
- path: `.github/workflows/security.yml`
  - reason: The canonical source repository retains authority for the release and security automation surface during this bounded promotion.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
  - consumed by: `(active)`
- path: `policy/automation-authority.toml`
  - reason: Canonical source now carries its explicit source-role policy from #679; this bounded decision retains that release-governance file while swarm keeps its distinct swarm-role policy.
  - source target: `6471f50912d645447ebda94c3b2f1babcc24dfef`
  - swarm target: `b75893463c466990122d51b3290287b107b0388e`
  - decision receipt: `EffortlessMetrics/shiplog-swarm#334` at `b75893463c466990122d51b3290287b107b0388e`
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
