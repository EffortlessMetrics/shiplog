# Current shiplog-swarm Promotion

This is the maintained operational summary for the latest source promotion.
Refresh it in the same source promotion PR by following
[`implementation-plan.md`](implementation-plan.md); it is not independent
execution authority.

**Status:** completed; approved source governance follows the promotion
**Promoted swarm head:** `c4fdba223d1c5c5b99a95b159ab8123d83d4b842`
**Source promotion:** `EffortlessMetrics/shiplog#655`
**Source merge commit:** `160d430f1a5af338537e35ff98b8ddda14d4673c`
**Source governance:** `EffortlessMetrics/shiplog#656`

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

## Truth hierarchy

1. Git refs and ancestry
2. GitHub PR and check state
3. This maintained summary for the latest completed promotion
4. Historical promotion receipts in `implementation-plan.md`

## Topology boundary

- Product development remains authoritative in `EffortlessMetrics/shiplog-swarm`.
- Source promotion uses a regular merge commit; do not squash.
- Release authority, tags, publishing, signing, and release workflows remain in `EffortlessMetrics/shiplog`.

## Next action

Prepare the next source promotion for the pending swarm range with `cargo xtask promote --swarm-sha $(git rev-parse swarm/main)`. Carry these receipts in the next substantive swarm PR; do not open a receipt-only refresh PR.
