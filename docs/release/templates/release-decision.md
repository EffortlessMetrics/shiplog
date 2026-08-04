# shiplog X.Y.Z — Release Decision

**Release target:** `vX.Y.Z`
**Theme:** `<one user-facing sentence>`
**Status:** proposed | ready to stage | shipped | cancelled

## Decision

State the actual release/no-release decision and why this version earns a
public release. Do not use control-plane completion alone as the justification.

## Included boundary

- `<merged user-visible capability or meaningful fix>`
- `<distribution or platform change>`
- `<compatibility or safety improvement>`

## Explicitly deferred

- `<non-blocking follow-up>`
- `<platform or provider surface not claimed by this release>`

## Compatibility and migration

Describe configuration, schema, CLI, artifact, package, or workflow
compatibility. State `none` when there is no user action.

## Safety and proof decision

Record the release-specific proof boundary:

- required source/swarm CI;
- fail-closed behavior retained or changed;
- security or privacy claims actually exercised;
- supported platform artifact matrix;
- signing/notarization status, when applicable; and
- checks intentionally advisory or unavailable.

## Distribution decision

Record which public channels are in scope:

- crates.io;
- GitHub Release assets;
- versionless installers;
- Homebrew;
- Scoop; and
- any channel explicitly deferred.

## Claim boundary

State what this decision does **not** authorize or prove. A ready decision does
not itself tag, publish, sign, upload, or make a release public.
