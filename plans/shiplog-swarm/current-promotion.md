# Current shiplog-swarm Promotion

**Status:** completed; approved source governance follows the promotion
**Promoted swarm head:** `c4fdba223d1c5c5b99a95b159ab8123d83d4b842`
**Source promotion:** `EffortlessMetrics/shiplog#655`
**Source governance:** `EffortlessMetrics/shiplog#656`

## Included work

- `EffortlessMetrics/shiplog-swarm#238` — add the bounded, idempotent source-promotion preparation entrypoint.
- `EffortlessMetrics/shiplog#655` — promote the exact proven swarm head with a regular merge commit.
- `EffortlessMetrics/shiplog#656` — keep source dependency automation verification-only without changing the promoted product tree.

## Topology boundary

- Product development remains authoritative in `EffortlessMetrics/shiplog-swarm`.
- `policy/source-only-paths.toml` explicitly approves the source-only `.github/dependabot.yml` governance surface.
- Unknown source-only paths and all changed product paths remain drift.
- Source promotion uses a regular merge commit; do not squash.
- Release authority, tags, publishing, signing, and release workflows remain in `EffortlessMetrics/shiplog`.

## Verification

```bash
rtk git fetch origin --prune
rtk git fetch swarm --prune
rtk git merge-base origin/main swarm/main
rtk cargo xtask repo-contract-report
rtk git diff --check
```

## Next action

Use the next substantive swarm PR to carry these receipts. Do not create a
receipt-only refresh PR. The bounded-manifest follow-up remains issue #241.
