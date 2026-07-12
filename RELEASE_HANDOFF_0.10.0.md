# shiplog 0.10.0 - Release Execution Handoff

**Shipped tag:** `v0.10.0`
**Status:** released on 2026-07-11
**Readiness ledger:** [`docs/release/0.10.0-readiness.md`](docs/release/0.10.0-readiness.md)
**Release decision:** [`docs/release/0.10.0-release-decision.md`](docs/release/0.10.0-release-decision.md)

This handoff records the bounded `0.10.0` release path and its completed
public-state proof.

## Release contents

- PR #619: source listing and comment-preserving source enable/disable toggles.
- PR #625: HTTPS-only validation for opt-in OpenAI-compatible LLM endpoints.
- Workspace and package metadata aligned to `0.10.0`.
- Changelog, roadmap, readiness, and decision artifacts aligned to the same
  release scope.

## Final preflight from merged main

```text
rtk git switch main
rtk git pull --ff-only
rtk git status --short
rtk git tag -l v0.10.0
rtk git ls-remote --tags origin v0.10.0
rtk gh release view v0.10.0

rtk cargo fmt --all -- --check
rtk cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
rtk cargo test --workspace --all-features --locked
rtk cargo xtask check-no-panic-family --mode blocking-allowlist
rtk cargo xtask check-policy-schemas
rtk cargo xtask check-file-policy --mode blocking-allowlist
rtk cargo xtask check-generated --mode blocking-allowlist
rtk git diff --check
rtk bash scripts/package-proof.sh
rtk bash scripts/publish-dry-run.sh
rtk cargo publish -p shiplog --dry-run --locked
rtk bash scripts/check-release-hold.sh v0.10.0
```

Stop before tagging if any preflight command fails or if the tag or release
already exists unexpectedly.

## Release execution

```text
rtk git tag -a v0.10.0 -m "shiplog v0.10.0"
rtk git push origin v0.10.0
```

After `release.yml` proves the package, assets, validation, and release tests:

```text
rtk cargo publish -p shiplog --locked
rtk gh release edit v0.10.0 --draft=false --latest
rtk cargo install shiplog --version 0.10.0 --locked --force
rtk shiplog --version
rtk shiplog --help
rtk cargo search shiplog --limit 5
rtk gh release view v0.10.0 --json tagName,isDraft,isPrerelease,publishedAt,assets,url
```
