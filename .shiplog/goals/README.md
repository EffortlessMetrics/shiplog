# Shiplog Goal Manifests

This directory is the repo-local control plane for active agent work. It is
not shiplog runtime configuration and should not be read by the product.

Goal manifests tell Codex, Droid, or another repository agent what is actually
being worked now:

- objective;
- linked proposal, specs, ADRs, plans, and issues;
- ready work items;
- proof commands;
- stop conditions;
- archive status after completion.

## Active Goal

Use `active.toml` for the current lane once a plan PR creates it. Keep it
small enough that an agent can read it first and know the next safe move.

Do not store secrets, tokens, private user data, or machine-local paths in goal
manifests. Use repository-relative paths and public issue or PR references.

## Archive

When a lane completes or is superseded, move the prior manifest under
`archive/` and record why. The archive is for execution history, not product
documentation.

## Relationship To Other Artifacts

Goal manifests do not define behavior. They point to the source of truth:

- proposals for why the lane exists;
- specs for behavior contracts;
- ADRs for durable architecture decisions;
- plans for sequencing and proof;
- policy ledgers for exceptions and receipts;
- guides for user-facing how-to.
