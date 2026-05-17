# SHIPLOG-ADR-0008: Doctor Is Setup Readiness, Not Intake

Status: accepted
Date: 2026-05-17
Related proposal:
[`SHIPLOG-PROP-0005-guided-setup-doctor`](../proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md)
Related spec:
[`SHIPLOG-SPEC-0007-setup-readiness`](../specs/SHIPLOG-SPEC-0007-setup-readiness.md)
Related repair spec:
[`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
Related packet-quality spec:
[`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)

## Context

The review-ready packet lane made Shiplog useful after a rough first run:
intake records receipts, repair plan exposes safe fixes, journal repair can add
local evidence, repair diff proves what changed, packet quality explains
readiness and claim candidates, and share explain keeps posture read-only before
rendering.

The post-0.8 soak also showed that many confusing moments are setup problems
that surface too late:

- malformed manual journals block repair commands after intake;
- missing provider tokens appear as packet caveats instead of setup facts;
- missing redaction keys are discovered at share time;
- old or partial reports need rerun guidance before review-ready signals are
  available;
- top-level handoffs are safest when commands clearly say whether they read or
  write.

That creates pressure to make `shiplog doctor --setup` smarter. The risk is
that doctor becomes a second intake path: probing providers, inferring evidence
health, scraping packet Markdown, or mutating setup automatically. That would
blur Shiplog's receipt boundaries and make diagnostics harder to trust.

Shiplog already has durable boundaries:

- [`SHIPLOG-ADR-0001`](SHIPLOG-ADR-0001-ingest-output-is-receipt-boundary.md)
  puts adapter evidence at the intake receipt boundary;
- [`SHIPLOG-ADR-0006`](SHIPLOG-ADR-0006-repair-actions-are-receipt-derived.md)
  requires repair actions to come from report receipts;
- [`SHIPLOG-ADR-0007`](SHIPLOG-ADR-0007-claim-candidates-are-evidence-scaffolds.md)
  keeps packet quality as evidence scaffolding instead of generated review
  prose;
- [`SHIPLOG-SPEC-0007`](../specs/SHIPLOG-SPEC-0007-setup-readiness.md)
  defines setup readiness as a prerequisite signal.

Doctor needs the same separation.

## Decision

`shiplog doctor --setup` and `shiplog sources status` are setup readiness
surfaces. They may inspect local setup prerequisites and durable local
receipts, but they must not collect new evidence.

Doctor may inspect:

- `shiplog.toml`;
- `manual_events.yaml`;
- local Git repository path and readability;
- configured source enablement and disablement;
- environment variable presence for credential-backed sources;
- cached config or source metadata that already exists locally;
- latest report metadata only when the command explicitly asks for latest-report
  context;
- share profile configuration and redaction-key presence.

Doctor must not:

- query GitHub, GitLab, Jira, Linear, or other provider APIs by default;
- mutate provider state;
- mutate config or manual journal files by default;
- render manager or public share artifacts;
- scrape `intake.report.md` or `packet.md` as machine input;
- infer source evidence freshness, packet readiness, or claim strength from
  token presence;
- generate performance-review prose;
- treat missing optional provider credentials as fatal to local-only setup.

`shiplog init --guided` is the setup writer. It may create or update local
setup files only because the user invoked a writer. Doctor and source status
remain no-write by default.

`shiplog sources status` is the source-only projection of setup readiness. It
must use the same typed setup model as doctor rather than a parallel source
classifier.

Read/write posture is part of the decision. Setup readiness next actions must
say whether a command writes, and read-only commands should be recommended
before write-producing commands when both are relevant.

## Consequences

- Doctor becomes a safe first command before intake, repair, or share.
- Setup readiness can tell the user that local Git is ready, manual evidence is
  blocked, GitHub is unavailable, Jira is disabled, or manager share lacks a
  redaction key without performing intake.
- Missing provider tokens can be surfaced as setup facts without implying the
  provider has fresh, stale, skipped, or collected evidence.
- Repair plan can route setup-blocked repairs to doctor without offering
  impossible `journal add --from-repair` commands.
- Share readiness can be visible before rendering, while `share explain`,
  `share verify`, and share rendering keep their existing responsibilities.
- A future provider probe mode, OAuth connection check, or setup wizard must be
  explicit and must not weaken the default no-network/no-write doctor contract.
- Tests must prove no-write behavior for doctor and source status, and must
  prove that setup status does not come from Markdown scraping.

## Alternatives Considered

### Make doctor a dry-run intake engine

Rejected. A dry-run intake would blur setup readiness with evidence collection,
provider availability, freshness, and packet quality. Intake owns evidence
receipts; doctor owns prerequisites.

### Probe providers by default

Rejected. Live provider probing introduces network latency, rate limits,
authentication failure modes, and secret-handling risk. Token presence can
explain setup availability, but it cannot prove source evidence health.

### Scrape packet Markdown for setup state

Rejected. Markdown is a human rendering. Report JSON, config, local files, and
explicit setup receipts are the machine boundary. Scraping Markdown would make
phrasing and section order accidental APIs.

### Auto-fix config from doctor

Rejected. Diagnostics should be safe to run repeatedly without mutation. Guided
init and explicit repair commands are the write surfaces.

### Treat missing optional providers as fatal

Rejected. Local-only and manual-only users are valid users. Missing optional
tokens should produce caveats or setup guidance, not block a useful local setup.

### Fold doctor into release execution

Rejected. Guided setup is a product lane, not release approval. The paused
0.9.0 release decision remains separate from doctor implementation.

## Affected Specs, Plans, Tests, And Schemas

- [`SHIPLOG-SPEC-0007-setup-readiness`](../specs/SHIPLOG-SPEC-0007-setup-readiness.md)
  defines the setup readiness model, status vocabulary, allowed inputs, and
  acceptance criteria.
- [`SHIPLOG-PROP-0005-guided-setup-doctor`](../proposals/SHIPLOG-PROP-0005-guided-setup-doctor.md)
  defines the product lane and non-goals.
- [`SHIPLOG-SPEC-0005-evidence-repair-loop`](../specs/SHIPLOG-SPEC-0005-evidence-repair-loop.md)
  remains the repair receipt and repair handoff contract.
- [`SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates`](../specs/SHIPLOG-SPEC-0006-packet-quality-and-claim-candidates.md)
  remains the packet readiness, claim candidate, and share posture contract.
- Future doctor model and CLI tests should prove ready, ready-with-caveats,
  needs-setup, blocked, disabled, unavailable, malformed manual journal,
  missing credential, no-write, and read-first next-action behavior.
- Future repair integration tests should prove setup-blocked repairs route
  through doctor before impossible local or provider repair commands.
- Future share tests should prove doctor can report manager/public setup
  readiness without writing profile artifacts.
