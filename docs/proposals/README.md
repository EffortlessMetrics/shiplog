# Shiplog Proposals

Proposals explain why a product or repository lane should exist before the
repo commits to behavior, architecture, or PR sequencing.

Use a proposal when shiplog needs to decide:

- which user pain is worth solving;
- who benefits from the lane;
- what success looks like;
- what alternatives were rejected;
- which release target or issue queue owns the work.

Proposals are not specs, ADRs, or implementation plans. They can recommend
direction, but they do not define the final behavior contract or the PR order.

## Naming

Use:

```text
SHIPLOG-PROP-0001-short-slug.md
```

Keep identifiers stable. If a proposal is superseded, add a status note to the
old proposal and link to the replacement rather than renaming the file.

## Required Shape

Each proposal should include:

- status;
- problem statement;
- target users;
- success criteria;
- non-goals;
- alternatives considered;
- linked specs, plans, issues, or release targets when they exist.

## Relationship To Other Artifacts

```text
README / guide        teaches the user what to do
proposal              explains why the lane exists
spec                  defines what must be true
ADR                   records durable architecture decisions
plan                  sequences PRs and proof commands
active goal manifest  tells agents what is being worked now
policy ledger         records exceptions, proof, or enforcement posture
```

If two artifacts disagree, use the artifact with the narrower authority:
specs win for behavior contracts, ADRs win for durable architecture decisions,
plans win for sequencing, and policy ledgers win for policy exceptions or
proof receipts.
