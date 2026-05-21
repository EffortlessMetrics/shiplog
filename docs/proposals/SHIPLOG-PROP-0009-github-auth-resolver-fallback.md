# SHIPLOG-PROP-0009: Seamless GitHub Credential Resolution (Env + gh CLI)

Status: proposed
Owner: product/docs
Created: 2026-05-20
Target release: 0.9.x follow-up lane
Linked lane: [`SHIPLOG-PROP-0007-github-activity-harvest`](./SHIPLOG-PROP-0007-github-activity-harvest.md)

## Summary

Shiplog should accept the GitHub credential state users already have instead of
forcing a personal access token (PAT)-only setup. For 0.9+, the intended user
path is:

```text
Download shiplog.
Run shiplog doctor --setup.
If gh is already logged in, shiplog can use it.
If not, shiplog gives the shortest safe next action.
```

PATs remain supported, but they are no longer the only happy path.

## Problem Statement

GitHub activity harvest is now a real operating loop:

```text
plan -> scout -> run -> resume -> ledger -> report/merge
```

When this expensive path starts with “manually create and paste a PAT,” many
users bounce before proving value. This is product friction, not merely setup
noise.

Most developer users already have one of two states:

- environment token set for automation; or
- authenticated `gh` CLI session.

Shiplog should consume those existing credentials safely and receipt the source
used, without becoming an OAuth/login product in this release.

## Target Users

Primary:

- users running `shiplog doctor --setup` as first-run onboarding;
- users running GitHub activity scout/run who already use `gh`;
- operators and agents who need deterministic, non-secret receipts of auth
  state.

Secondary:

- maintainers extending the same resolver pattern later to OAuth/keychain;
- reviewers validating that credential ergonomics improved without weakening
  receipt hygiene.

## Proposed Direction

### Auth Source Precedence

Resolve GitHub credentials in this order:

```text
1. GH_TOKEN
2. GITHUB_TOKEN
3. GH_ENTERPRISE_TOKEN
4. GITHUB_ENTERPRISE_TOKEN
5. gh CLI login (gh auth token + gh auth status --json hosts)
6. unavailable
```

Product behaviors:

- explicit env tokens override stored `gh` credentials;
- enterprise token vars are treated as distinct sources;
- `gh` is credential discovery only (not request execution);
- unresolved auth yields actionable setup guidance.

### Product Rule

Shiplog should accept the credential state the user already has, but must
receipt which credential source it used.

Receipts must include source metadata only, for example:

```json
{
  "auth": {
    "source": "gh_cli",
    "host": "github.com",
    "account": "EffortlessSteven"
  }
}
```

Never write token values, authorization headers, or raw `gh auth token` output
into config, receipts, logs, or status artifacts.

### UX Outcomes

- `doctor --setup` reports `ready | needs_setup | unavailable` with next step;
- `sources status` includes concise auth source projection;
- `github activity plan` remains available without mandatory token resolution;
- `github activity scout/run` resolve auth once at start, then run with
  shiplog’s existing HTTP/cache/ledger path;
- receipt-reader commands (`status/report/merge`) do not shell out to `gh`.

### Explicit Auth Inspection

Add a dedicated command:

```text
shiplog auth github status
shiplog auth github status --json
```

This command should explain source/host/account/token availability (without
printing token values) and recommended next action.

## Success Criteria

- Users with existing `gh` login can run GitHub intake/activity without creating
  a new PAT.
- Environment token precedence is deterministic and documented.
- Setup outputs explicitly identify auth source and host without exposing
  secrets.
- Activity receipts capture auth source metadata only.
- `gh` is not used as the API request engine for harvest pagination/details.

## Non-Goals

- implementing OAuth/device/browser login in shiplog;
- storing or refreshing provider tokens;
- printing token values or scopes by default;
- shelling out to `gh api ...` for harvest execution;
- mutating provider account state from shiplog.

## Alternatives Considered

### Keep PAT-only

Rejected: highest setup friction for users who are already authenticated via
`gh`, especially in first-run `doctor --setup` flow.

### Use `gh api` as transport

Rejected: loses shiplog-native control of pagination, cache behavior, and API
ledger accounting; adds subprocess coupling in hot paths.

### Build OAuth now

Deferred: likely long-term best UX, but significantly larger product/security
surface than needed for current 0.9.x ergonomics gap.

## Proposed Sequencing (Follow-up Artifacts)

1. Spec: define resolver contract, precedence, receipt fields, failure modes.
2. ADR: record that shiplog consumes existing credentials and does not own login.
3. Implementation: resolver + doctor/sources/activity integration.
4. Product proof tests with fake `gh` executable and secret redaction checks.
5. Guide updates for seamless setup (`gh auth login` and env fallback).

## Open Questions

- Should enterprise-host selection be explicit in config when both dotcom and
  GHES credentials exist?
- For multi-account `gh` setups, should “needs_setup” require actor-match checks
  before expensive runs?
- Should `github activity plan` receipt `auth_source: not_required_for_plan` by
  default, or attempt non-blocking detection for operator visibility?
