# 0.10.0 Source-of-Truth Rollout Plan

## Work item: docs-source-of-truth-scaffold

Status: ready
Linked proposal: SHIPLOG-PROP-0008
Linked spec: SHIPLOG-SPEC-0010
Linked ADR: none
Blocks: policy-doc-artifact-ledger
Blocked by: none
Branch: docs/source-of-truth-stack
Issue:
PR:

### Goal

Add linked scaffolding artifacts for proposals, specs, ADRs, plans, active goals,
support tiers, and policy ledgers.

### Production delta

Documentation, templates, goals manifests, and policy ledger files only.

### Non-goals

No runtime behavior changes or broad refactors.

### Acceptance

Artifact structure exists and files cross-link to the source-of-truth stack.

### Proof commands

```bash
git diff --check
```

### Rollback

Revert this PR as a single documentation/policy slice.

### Claim boundary

Does not prove runtime feature behavior; only repo governance and traceability surfaces.
