# Automation Authority: Where Automated Changes May Originate

Owner: repo-infra/release
Status: enforced
Policy ledger: [`policy/automation-authority.toml`](../../policy/automation-authority.toml)
Checker: `cargo xtask check-automation-authority --repository-role <swarm|source>`
Related: [`SHIPLOG-ADR-0011`](../adr/SHIPLOG-ADR-0011-swarm-is-dev-landing-zone-not-release-surface.md),
[`policy/source-only-paths.toml`](../../policy/source-only-paths.toml)

## Why this boundary exists

Shiplog runs a two-repository model:

- **`EffortlessMetrics/shiplog-swarm`** — the development trunk. All product,
  dependency, workflow, security-remediation, and documentation changes intended
  for release land here first.
- **`EffortlessMetrics/shiplog`** — the release surface. It verifies the promoted
  product tree and retains exclusive authority over tags, crates.io publication,
  GitHub Releases, signing, and package-manager updates.

The model only holds if product-changing automation targets **one** repository.
When bots can open product PRs in both repos, the same code path gets duplicated
review, source picks up commits that never flowed through promotion, and the
promotion-only relationship between the two repos breaks. This happened in
practice: parallel source and swarm security-scan PRs for the same code path, and
a source Dependabot configuration that had to be disabled separately.

The invariant is therefore: **`shiplog-swarm` is the only repository where routine
automation may propose product changes.** `shiplog` may verify and may execute
explicitly authorized releases, but its routine automation cannot mutate product
files.

## The authority matrix

Each automation surface has a declared effect per repository. These effects are the
enforceable contract in [`policy/automation-authority.toml`](../../policy/automation-authority.toml);
this table is the human-readable mirror.

| Automation surface     | `shiplog-swarm` (swarm) | `shiplog` (source)             |
| ---------------------- | ----------------------- | ------------------------------ |
| Dependency updates     | `product-pr`            | `verification-only`            |
| Security remediation   | `product-pr`            | `verification-only`            |
| Scheduled security     | `product-pr`            | `check-artifact-or-handoff`    |
| Review bots            | `review-comment`        | `review-comment`               |
| Promotion              | `prepare-source-pr`     | `merge-checkpoint`             |
| Release execution      | `forbidden`             | `explicitly-authorized`        |
| Emergency hotfix       | `product-pr`            | `authorized-only-then-backport`|

Effect meanings:

- **`product-pr`** — may open a branch/PR that changes product, dependency,
  workflow, or documentation files. Only the swarm repo grants this for routine
  automation.
- **`verification-only`** — may inspect the promoted tree and fail or report, but
  may not commit or push product changes.
- **`check-artifact-or-handoff`** — may run scheduled analysis and record findings
  as a workflow check or uploaded artifact, or open an issue that hands the
  remediation off to swarm. It may not fix product files in place.
- **`review-comment`** — may publish review comments/annotations only; no product
  writes.
- **`prepare-source-pr`** — swarm-side promotion tooling (`cargo xtask promote`)
  prepares an exact-SHA source promotion branch/PR but never merges it.
- **`merge-checkpoint`** — the source side accepts a promotion as a regular merge
  commit (never squashed); this is the only routine way product changes enter
  source.
- **`forbidden`** — release execution never runs from swarm.
- **`explicitly-authorized`** — release execution runs only from source, only under
  an explicit human-authorized action, with the minimum write permissions the
  release jobs actually need.
- **`authorized-only-then-backport`** — see the emergency path below.

## How the boundary is enforced

`cargo xtask check-automation-authority --repository-role <swarm|source>` inspects
the repository against the matrix and fails closed on any violation. It runs pinned
to the repository's declared role as part of `cargo xtask ci-small`, so a healthy
PR proves the boundary on every change.

The role cannot be spoofed inside GitHub Actions: when `GITHUB_ACTIONS=true`, the
checker binds the role to the immutable `GITHUB_REPOSITORY` identity
(`shiplog-swarm` → swarm, `shiplog` → source) and rejects any requested role that
disagrees, any unknown repository, and any fork identity.

The checker verifies, per role:

- **Dependabot** — swarm must retain authoritative update entries; source must
  declare an empty `updates:` list.
- **Workflow permissions** — source routine workflows may not grant
  `contents: write` or any other `write` scope; only the source release-authority
  jobs (`create-release`, `upload-assets` in `release.yml`) may declare
  `contents: write`. Swarm's `release.yml` stays verification-only
  (`contents: read`).
- **Mutation paths** — workflow bodies are scanned for mutation primitives
  (`git push`, `gh pr create`, `create-pull-request`, app-token/PAT minting,
  `cargo publish`). Release operations (`softprops/action-gh-release`,
  `gh release create`) are allowed only in the source release-authority jobs;
  alternate credential or push paths are allowed only in swarm non-release
  workflows.
- **Matrix integrity** — the policy ledger itself is validated for the exact set of
  automation rows and effects, with no duplicates, omissions, or contradictions.
- **Source routine-bot guard** — source-role validation requires
  `source-automation-guard.yml` to use `pull_request_target` for opened, reopened,
  and synchronized PRs, match the `dependabot[bot]` and `factory-droid[bot]`
  identities, fail with `exit 1`, expose a required `workflow_dispatch`
  `author_login` input for synthetic actor proof, and remain metadata-only
  (`contents: read`, `pull-requests: read`, no checkout or named secrets).

Intentional per-repository configuration differences (for example, source's empty
Dependabot list or its release-writer `release.yml`) are classified as approved
release governance in [`policy/source-only-paths.toml`](../../policy/source-only-paths.toml)
so `repo-contract-report` treats them as governance, not product drift.

## Source verification and durable handoff

Source security and dependency jobs are verification-only. When a source
verification run finds a problem in the promoted tree it must produce a **durable
handoff to swarm** rather than fixing the tree in place:

1. Fail (or report) the check so the promoted tree is visibly unsafe.
2. Record the finding as a workflow check result or uploaded artifact, and/or open
   an issue pointing at the swarm remediation.
3. Leave every product file untouched.

The source-side [`Source Automation Guard`](../../.github/workflows/source-automation-guard.yml)
also fails closed for routine product PRs authored by `dependabot[bot]` or
`factory-droid[bot]`. It uses `pull_request_target` only to read pull-request
metadata, never checks out the untrusted head, and grants no write permission.
An authorized maintainer can use `workflow_dispatch` with `author_login` set to
each bot identity to exercise the same reject path, and with an ordinary login
to exercise the allowed path. Those runs prove actor classification and the
workflow's fail-closed behavior; they do not prove GitHub branch protection or
that an unprotected source branch cannot be merged.
The guard is intentionally repository-gated, so the same workflow can travel
through the swarm promotion tree without rejecting the swarm's authoritative
automation. A failed guard is a handoff signal, not a source-side mutation path.
The source-role automation-authority check validates this guard's static contract;
branch protection and required-check configuration remain separate GitHub controls.

Swarm then originates the fix as a normal `product-pr`, and the fix reaches source
only through the next promotion `merge-checkpoint`.

## Emergency hotfix exception path

Emergencies do not suspend the boundary; they route through it faster.

**Preferred path.** Create the fix in `shiplog-swarm` as a normal focused PR, take
it through `Shiplog Rust Small Result`, squash-merge it, and promote it to source
with `cargo xtask promote`. This keeps the single-trunk invariant intact even under
time pressure.

**Authorized source hotfix.** If a fix genuinely cannot wait for a swarm round trip
(for example, an active release-surface incident), an explicitly authorized human
may apply it directly to `shiplog`. This is the `authorized-only-then-backport`
effect and it carries two obligations:

1. The action must be explicitly human-authorized — no bot or routine automation
   may take this path.
2. The same change must be **back-ported into `shiplog-swarm` immediately**, before
   normal promotion resumes, so the two product trees re-converge and source does
   not accumulate un-promoted product commits.

Release execution itself (tags, crates.io, GitHub Releases, signing, package
managers, release credentials) is always source-only and always explicitly
authorized, in emergencies included. This document controls where automated changes
originate; it does not move or exercise release credentials.

## Changing this boundary

The matrix is a policy contract, not a convention. To change an effect:

1. Edit [`policy/automation-authority.toml`](../../policy/automation-authority.toml)
   and update the expected matrix in
   [`xtask/src/tasks/automation_authority.rs`](../../xtask/src/tasks/automation_authority.rs)
   in the same change.
2. Update this document's matrix and effect definitions to match.
3. If the change makes a file legitimately differ between source and swarm, record
   it in [`policy/source-only-paths.toml`](../../policy/source-only-paths.toml) with
   an owner, reason, classification, and `review_after` date.
4. Run `cargo xtask ci-small` so the pinned automation-authority check proves the
   new contract.
