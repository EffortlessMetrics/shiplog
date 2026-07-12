# shiplog 0.11.0 - Release Execution Handoff

**Release target:** `v0.11.0`
**Status:** release preparation
**Readiness ledger:** [`docs/release/0.11.0-readiness.md`](docs/release/0.11.0-readiness.md)
**Release decision:** [`docs/release/0.11.0-release-decision.md`](docs/release/0.11.0-release-decision.md)

This handoff records the bounded usability release path. Do not tag or publish
until the merged-main preflight and routed release checks pass.

## Release contents

- Objective-scoped intake readiness and independent share readiness.
- GitHub credential reuse from supported environment variables or `gh`.
- No-argument home, `next`, `update`, quick `add`, packet-first output, and
  default `open`.
- Direct share preflight, `status --check`, verified installers, Homebrew, and
  Scoop.

## Execution

```text
rtk git tag -a v0.11.0 -m "shiplog v0.11.0"
rtk git push origin v0.11.0
rtk cargo publish -p shiplog --locked
rtk gh release edit v0.11.0 --draft=false --latest
```

Afterward, verify the installed crate, release assets/checksums, Linux,
Windows, Homebrew, Scoop, and the packaged first-packet smoke path.
