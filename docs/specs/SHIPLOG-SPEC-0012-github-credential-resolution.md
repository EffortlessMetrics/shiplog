# SHIPLOG-SPEC-0012: GitHub Credential Resolution

Status: accepted
Owner: product/security
Created: 2026-07-11
Related proposal:
[`SHIPLOG-PROP-0009-github-auth-resolver-fallback`](../proposals/SHIPLOG-PROP-0009-github-auth-resolver-fallback.md)
Related architecture decision:
[`SHIPLOG-ADR-0012-github-credentials-are-discovered-not-owned`](../adr/SHIPLOG-ADR-0012-github-credentials-are-discovered-not-owned.md)
Related setup contract:
[`SHIPLOG-SPEC-0007-setup-readiness`](SHIPLOG-SPEC-0007-setup-readiness.md)
Related objective contract:
[`SHIPLOG-SPEC-0010-objective-scoped-setup-readiness`](SHIPLOG-SPEC-0010-objective-scoped-setup-readiness.md)

## Purpose

Shiplog should use an authentication method that is already available to the
developer without turning Shiplog into a credential store. GitHub credential
resolution is therefore a process boundary: secret material may be used by a
provider request, but only safe provenance metadata may cross into diagnostics,
receipts, JSON, or configuration.

## Scope

This spec owns:

- deterministic environment-variable and `gh` CLI credential precedence;
- GitHub host selection for dotcom and enterprise installations;
- safe authentication metadata and diagnostic reasons;
- the boundary between credential discovery and GitHub request transport;
- the read-only auth status and setup-readiness projections;
- account mismatch handling during readiness and intake.

Out of scope:

- OAuth, device authorization, or operating-system keychain integration;
- persisting tokens or changing provider credentials;
- replacing Shiplog's native HTTP, cache, or API accounting path;
- changing the configured GitHub actor silently;
- provider API calls from doctor, source status, or auth status.

## Host Selection

The target host comes from `sources.github.api_base`:

- absent or `https://api.github.com` means `github.com`;
- an enterprise API base uses its URL host, such as `github.example.com`;
- an invalid or ambiguous configured host is unavailable with a safe setup
  action rather than guessed.

The resolver must never choose credentials for a different host. When no
explicit host is configured, dotcom is the default. If `gh` reports multiple
authenticated hosts and no configured host identifies one unambiguously, the
resolver returns an actionable ambiguity diagnostic instead of prompting or
choosing arbitrarily.

## Resolution Order

For the selected host, candidates are considered in this order:

```text
1. GH_TOKEN
2. GITHUB_TOKEN
3. GH_ENTERPRISE_TOKEN
4. GITHUB_ENTERPRISE_TOKEN
5. authenticated gh CLI credential for the selected host
6. unavailable
```

The generic variables apply to dotcom and the enterprise variables apply to
enterprise hosts. A variable for another host family is ignored, not used as
a cross-host fallback. The first non-empty applicable variable wins, and a
usable environment credential prevents any `gh` subprocess from running.

`gh` discovery uses credential discovery commands only:

```text
gh auth status --json hosts
gh auth token --hostname <selected-host>
```

Shiplog must never use `gh api` as its request transport. Missing `gh`, a
logged-out host, malformed output, subprocess failure, and timeout are safe
unavailable reasons. Their diagnostics must not include raw subprocess output.

## Safe Metadata

Every resolution produces safe metadata, whether a credential is available or
not. The metadata may contain:

```json
{
  "source": "gh_cli",
  "host": "github.com",
  "account": "steven",
  "availability": "ready"
}
```

`source` identifies the selected non-secret source, including the applicable
environment variable name or `gh_cli`. `host` is the selected GitHub host.
`account` is optional and may come from `gh auth status`; it is advisory until
the provider's `/user` response is obtained during intake. `availability` is
`ready` or `unavailable`. An unavailable result may include a stable,
secret-free diagnostic reason suitable for human output and JSON.

The following must never appear in metadata, configuration, receipts, reports,
errors, debug output, or command output:

- token values or authorization headers;
- raw `gh` stdout or stderr;
- serialized secret-bearing resolver objects;
- redaction keys or other unrelated credentials.

The in-memory secret-bearing value must not derive ordinary `Debug` or
`Serialize`. If a custom debug representation exists, it must identify only
that a credential is present.

## Consumer Contracts

`shiplog auth github status` and `--json` are read-only. They resolve and
report safe metadata but do not call GitHub, mutate configuration, or invoke
`gh api`.

`shiplog doctor --setup` and `shiplog sources status` may include the same safe
metadata in their typed projections. They remain no-network, no-write
readiness commands. Missing GitHub credentials are an intake caveat when
local/manual intake can proceed, not a fatal share or intake result by
themselves.

Provider intake and activity execution resolve once at the start of the
operation, retain the secret in memory only, and pass it to the existing native
GitHub HTTP/cache path. Activity planning, status, reports, merge operations,
and ordinary receipt readers do not resolve credentials or invoke `gh`.

During intake, the GitHub `/user` result is authoritative for the authenticated
account. If it differs from a configured actor or advisory `gh` account,
Shiplog warns before an expensive harvest and does not silently rewrite the
configured actor.

## Acceptance Criteria

- All four environment variables and host applicability are deterministic.
- Environment credentials take precedence over `gh`, and winning env
  resolution does not invoke `gh`.
- Dotcom, enterprise, missing, logged-out, malformed, timeout, and ambiguous
  host cases produce safe metadata and actionable diagnostics.
- `gh` is used only for credential discovery; Shiplog's native HTTP path
  remains the request transport.
- Auth status, doctor, and source status are read-only and do not contact the
  GitHub API.
- Provider execution resolves once and retains tokens only in memory.
- Account mismatch is advisory before intake and provider-authoritative during
  intake, without silent actor mutation.
- Secret sentinels are absent from stdout, stderr, JSON, configuration,
  receipts, reports, and debug representations.

## Proof Mapping

- The resolver unit tests cover precedence, host selection, metadata, and
  secret-bearing type behavior.
- Fake-`gh` integration tests cover missing executable, logout, malformed
  output, timeout, multiple hosts, enterprise hosts, and subprocess avoidance
  when an environment variable wins.
- Auth status and setup tests prove no GitHub API calls and no local writes.
- Intake/activity tests prove native HTTP transport, one resolution per run,
  safe auth receipt metadata, and account mismatch warnings.
- The setup-readiness schema and examples must be updated together if auth
  metadata is added to the machine contract.
- A release-binary smoke test must prove the PAT-free first packet with a fake
  authenticated `gh` session and no token environment variables.

## Compatibility And Migration

Existing `GITHUB_TOKEN` behavior remains supported. Existing explicit GitHub
disablement and configured actors remain authoritative. Credential discovery
must not write tokens or rewrite existing provider filters, owners, comments,
or formatting. New safe auth fields are additive; existing setup and activity
consumers must continue to parse documents without them.
