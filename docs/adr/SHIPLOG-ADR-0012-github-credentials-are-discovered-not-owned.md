# SHIPLOG-ADR-0012: GitHub Credentials Are Discovered, Not Owned

Status: accepted
Date: 2026-07-11
Related proposal:
[`SHIPLOG-PROP-0009-github-auth-resolver-fallback`](../proposals/SHIPLOG-PROP-0009-github-auth-resolver-fallback.md)
Related spec:
[`SHIPLOG-SPEC-0012-github-credential-resolution`](../specs/SHIPLOG-SPEC-0012-github-credential-resolution.md)
Related setup architecture:
[`SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake`](SHIPLOG-ADR-0008-doctor-is-setup-readiness-not-intake.md)

## Context

GitHub setup currently treats an environment token and explicit actor
configuration as the normal path. That is safe but makes a developer who is
already authenticated with the GitHub CLI create another token or edit TOML
before Shiplog can collect evidence.

Shiplog also has strict receipt and sharing boundaries. Reusing an existing
credential must not add token material to configuration, receipts, diagnostics,
or reports, and it must not replace Shiplog's native request and accounting
path with an opaque CLI transport.

## Decision

Shiplog owns a small, internal GitHub credential resolver that discovers a
credential from supported environment variables or an authenticated `gh` CLI
session. It returns an in-memory secret for provider execution separately from
safe metadata describing source, host, account, availability, and a diagnostic
reason.

The resolver selects credentials for the configured GitHub host, follows the
host-aware precedence in
[`SHIPLOG-SPEC-0012`](../specs/SHIPLOG-SPEC-0012-github-credential-resolution.md),
and never persists the secret. `gh` is a discovery adapter only. Shiplog's
existing HTTP client, cache, pagination, and API accounting remain the sole
request path.

Readiness and status surfaces may resolve credentials for safe metadata, but
they remain no-network/no-write commands and never call the GitHub API.
Provider intake and activity execution resolve once at operation start. Plan,
report, merge, and receipt-reader commands do not resolve credentials.

The provider `/user` response remains authoritative for the account used by an
intake. Advisory account information from `gh` is useful for diagnostics but
cannot silently change an explicit Shiplog actor.

## Consequences

- Existing authenticated `gh` sessions can support a first packet without a
  new PAT or manual token storage.
- Environment-token users retain deterministic and backwards-compatible
  behavior.
- Secret handling is concentrated in one testable boundary.
- Receipts can explain authentication provenance without exposing credentials.
- The resolver must handle subprocess availability, timeout, malformed output,
  host ambiguity, and Windows process behavior.
- Provider execution must pass the resolved secret through existing native
  transport seams rather than invoking `gh api`.
- Future credential backends can implement the same safe metadata boundary
  without changing receipt consumers.

## Alternatives Considered

### Require a new personal access token

Rejected. It creates avoidable setup friction, duplicates credentials already
available to the developer, and makes the one-command local-first path less
credible.

### Use `gh api` for GitHub collection

Rejected. It would move request transport, pagination, caching, and API
accounting outside Shiplog's receipt boundary and make behavior harder to
test deterministically.

### Persist the discovered token in `shiplog.toml`

Rejected. Shiplog is not a credential store. Persisted secrets would expand
the data-loss and artifact-redaction surface without improving the evidence
model.

### Probe GitHub from doctor or auth status

Rejected. Readiness is a local setup projection. Provider probes add network
latency, rate-limit behavior, and authentication failure modes to commands
that should be safe and repeatable.

### Silently replace the configured actor with the authenticated account

Rejected. Identity selection is a user-owned configuration decision. A
provider response may report the authoritative account and warn about a
mismatch, but it must not rewrite intent.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0012-github-credential-resolution`](../specs/SHIPLOG-SPEC-0012-github-credential-resolution.md)
  defines the resolver and consumer contracts.
- [`SHIPLOG-SPEC-0007-setup-readiness`](../specs/SHIPLOG-SPEC-0007-setup-readiness.md)
  remains the no-network/no-write readiness boundary.
- Resolver, auth status, doctor, source status, intake, activity, and release
  smoke tests must prove the acceptance criteria in the related spec.
- Any new JSON fields require synchronized schema, examples, documentation,
  and contract-test updates.
