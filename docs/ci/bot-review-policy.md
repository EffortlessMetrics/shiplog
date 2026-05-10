# Bot Review Policy

Droid, CodeRabbit, and Gemini are part of the CI economy even though
they are not GitHub Actions compute. This document defines how bot
review fits into the lane model, when it blocks, and when it does not.

## The four rules

1. **Advisory unless explicitly required.** No bot review is a required
   check on `main` branch protection. Bot output is a signal, not a
   gate.
2. **Rate-limit behavior is non-blocking.** A bot that reports "rate
   limit exceeded" or fails to deliver a review is treated as a skip,
   not a failure. The PR is still mergeable.
3. **Critical findings need human decision.** A bot flagging "critical"
   or "block on critical" requires a maintainer to review the finding
   and either fix it, suppress it (with rationale), or override. The
   bot does not get the final word.
4. **Bot failure is not code failure.** If the bot infrastructure
   crashes or the API is down, that is an infrastructure problem, not
   a PR problem. The PR should not be blocked on infrastructure outside
   the repo's control.

## What runs today

| Bot | Workflow | Trigger | Lane | Posture |
|-----|----------|---------|------|---------|
| Factory Droid auto-review | `droid-review.yml` | PR open / sync / ready_for_review / reopened | PR fast advisory | Non-blocking; severity ≥ high; block on critical only |
| Factory Droid scheduled scan | `droid-security-scan.yml` | Mon weekly + dispatch | Weekly advisory | Non-blocking; severity ≥ medium |
| Factory Droid mention handler | `droid.yml` | `@droid` mentions | Out-of-band interactive | Non-blocking; user-triggered |
| GitGuardian | (GitHub-app installed; no workflow) | every push | PR fast | Blocking on critical secret findings (app config) |
| CodeRabbit | (GitHub-app installed; no workflow) | every PR | PR fast advisory | Non-blocking; rate-limited on free tier |
| gemini-code-assist | (GitHub-app installed; no workflow) | every PR | PR fast advisory | Non-blocking; daily quota |

## LEM / cost accounting

Bot lanes cost real money even though they don't burn GitHub Actions
minutes. The cost is tracked in
[`policy/ci-budget.toml`](../../policy/ci-budget.toml) under
`runner_multipliers.external_ai_review = 4.0` (4× Linux equivalent
billing) and surfaced in
[`lem-budgeting.md`](lem-budgeting.md). Specifically uncovered by LEM
proper but listed in `[external_cost_notes]`:

- Bot review tokens (Droid / CodeRabbit / Gemini per PR)
- Factory.ai Droid orchestration
- External model API calls

Treat these as known external spend; the LEM forecast in PR #146 reports
them as an estimated multiplier on PR cost rather than as wall-clock
minutes.

## When a bot finding blocks

The default rule is "advisory only." There are two narrow exceptions:

- **GitGuardian critical (secret leak).** A leaked secret is a
  shipping-blocker. The GitHub-app blocks on critical findings.
- **Factory Droid `block_on_critical = true` for security reviews.**
  Configured in `droid-review.yml`. A "critical" security finding
  blocks until a maintainer either resolves it or overrides.

Neither bot can promote a "high" or "medium" finding to blocking
without configuration change. If a high-severity finding is genuinely
shipping-blocking, a human reviewer holds the merge — the bot does not
escalate it automatically.

## When a bot fails

If `droid-review.yml` fails (API down, secret missing, etc.), the
workflow is configured with `continue-on-error: true` and the workflow
itself reports `success`. The PR remains mergeable.

If `coderabbitai` reports "rate limit exceeded" (as on PRs #140 and
#141 during the rollout), the `CodeRabbit` GitHub status check still
reports `success` — the rate limit is reported as a comment, not as a
status failure. The PR remains mergeable.

If `gemini-code-assist` reports "quota exceeded," same handling.

## When a bot is rate-limited and a critical lane needs review

Maintainer judgement. Options:

- Wait for rate-limit reset (usually <1 hour for CodeRabbit hourly,
  <24h for daily quotas).
- Apply the `@droid` mention to invoke the interactive bot.
- Proceed with human review only and note the missing bot signal in
  the PR description / merge commit.

There is no automatic mechanism to require bot review.

## Adding or removing a bot

- Adding a bot: requires an entry in
  [`policy/network-allowlist.toml`](../../policy/network-allowlist.toml)
  for the bot's destination + auth secret, plus an entry in
  [`policy/workflow-allowlist.toml`](../../policy/workflow-allowlist.toml)
  for the workflow. The new bot defaults to advisory; promotion to
  blocking is a separate PR with rationale.
- Removing a bot: requires removing the workflow + the policy entries
  in the same PR. Bot history (review comments) stays in the PR
  archive.

## See also

- [`policy/workflow-allowlist.toml`](../../policy/workflow-allowlist.toml) — droid workflow entries
- [`policy/network-allowlist.toml`](../../policy/network-allowlist.toml) — Droid / MiniMax / GitGuardian network entries
- [`policy/ci-budget.toml`](../../policy/ci-budget.toml) — `external_cost_notes` for non-LEM spend
- [`labels.md`](labels.md) — `ripr-waive` and other bot routing labels
- [`branch-protection.md`](branch-protection.md) — why bot checks are not required
