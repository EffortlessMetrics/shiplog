# shiplog X.Y.Z — Release Execution Handoff

**Release target:** `vX.Y.Z`
**Status:** preparing | staged | published | closed
**Release decision:** [`docs/release/X.Y.Z-release-decision.md`](docs/release/X.Y.Z-release-decision.md)
**Readiness ledger:** [`docs/release/X.Y.Z-readiness.md`](docs/release/X.Y.Z-readiness.md)

Copy this template to root `RELEASE_HANDOFF_X.Y.Z.md`. The links above are
written for that destination.

This handoff records the execution boundary and observed public state. Replace
all placeholders with evidence; do not mark it closed from intended commands.

## Exact release identities

| Identity | Value |
| --- | --- |
| Swarm candidate SHA | `<full sha>` |
| Source promotion PR / merge | `#<number>` / `<full sha>` |
| Source release-prep PR / merge | `#<number>` / `<full sha>` |
| Release source commit | `<full sha>` |
| Annotated tag | `vX.Y.Z` |
| Source Release workflow | `<run URL or ID>` |
| Staged candidate manifest | `<artifact/digest>` |
| Terminal candidate-ready aggregate | `<job/result>` |

## Published contents

Summarize the release in user language and link to the changelog. Keep internal
control-plane receipts in the readiness ledger rather than turning them into the
release story.

## Artifact receipt

| Artifact | Observed result |
| --- | --- |
| `shiplog-x86_64-unknown-linux-gnu` | `<asset/checksum/artifact digest>` |
| `shiplog-x86_64-apple-darwin` | `<asset/checksum/artifact digest>` |
| `shiplog-aarch64-apple-darwin` | `<asset/checksum/artifact digest>` |
| `shiplog-x86_64-pc-windows-msvc.exe` | `<asset/checksum/artifact digest>` |
| `SHA256SUMS.txt` | `<asset/checksum/artifact digest>` |

## Validation receipt

- Release preflight: `<result>`
- Release validation: `<result>`
- Four-platform first-use acceptance against one staged set: `<results>`
- Release-mode integration tests: `<result>`
- Terminal `Release Candidate Ready`: `<result>`
- crates.io install and `--version`: `<result>`
- GitHub release public-state verification: `<result>`
- Versionless installers: `<result>`
- Homebrew: `<PR/version/native validation>`
- Scoop: `<PR/version/native validation>`
- Signing/notarization: `<result or explicitly not configured>`

## Execution record

Record only commands actually executed, with the exact commit/tag context.
Typical final boundary:

```text
git tag -a vX.Y.Z -m "shiplog vX.Y.Z"
git push origin vX.Y.Z
# After exact-tag staged-candidate proof:
git checkout --detach vX.Y.Z
cargo publish -p shiplog --locked
gh release edit vX.Y.Z --draft=false --latest
```

Record that `cargo publish` ran from the exact detached tag checkout proven by
the source Release workflow, not from a later moving `main`.

## Exceptions, rollback, or yank

Record any failed attempt, replacement patch version, yank, draft deletion, or
public limitation. Release tags are immutable and published crate bytes are not
replaced.

## Residual work

- `<deferred product work>`
- `<package-channel follow-up>`
- `<source-only release change that must be ported back to swarm>`

## Claim boundary

State exactly what the completed release proves and what remains unverified.
Checksums are not a signing claim; skipped optional checks are not executed
proof; a public GitHub release does not by itself prove crates.io or package
channels.
