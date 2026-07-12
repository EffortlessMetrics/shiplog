# shiplog 0.11.0 - Release Execution Handoff

**Release target:** `v0.11.0`
**Status:** released on 2026-07-12
**Readiness ledger:** [`docs/release/0.11.0-readiness.md`](docs/release/0.11.0-readiness.md)
**Release decision:** [`docs/release/0.11.0-release-decision.md`](docs/release/0.11.0-release-decision.md)

This handoff records the bounded usability release path and its completed
public-state proof.

## Release contents

- Objective-scoped intake readiness and independent share readiness.
- GitHub credential reuse from supported environment variables or `gh`.
- No-argument home, `next`, `update`, quick `add`, packet-first output, and
  default `open`.
- Direct share preflight, `status --check`, verified installers, Homebrew, and
  Scoop.

## Completed execution

```text
rtk git tag -a v0.11.0 -m "shiplog v0.11.0"
rtk git push origin v0.11.0
rtk cargo publish -p shiplog --locked
rtk gh release edit v0.11.0 --draft=false --latest
```

Verified after publication:

- crates.io install reports `shiplog 0.11.0`;
- GitHub release assets and checksums are public;
- Linux and Windows release-install smoke paths pass;
- Homebrew native macOS/Linux validation passes;
- Scoop native Windows validation passes;
- release-mode integration tests pass.
