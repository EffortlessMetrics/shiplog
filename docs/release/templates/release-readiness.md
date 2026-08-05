# shiplog X.Y.Z — Release Readiness

**Release target:** `vX.Y.Z`
**Theme:** `<one user-facing sentence>`
**Status:** preparing | ready to stage | staged | shipped | blocked

## Candidate identities

| Identity | Value |
| --- | --- |
| Swarm candidate SHA | `<full sha>` |
| Source promotion PR | `EffortlessMetrics/shiplog#<number>` |
| Source promotion merge | `<full sha>` |
| Source release-prep PR | `EffortlessMetrics/shiplog#<number>` |
| Release source commit | `<full sha>` |
| Release tag | `vX.Y.Z` |

## Included release state

Summarize the user-visible release contents. Link to the release decision and
`CHANGELOG.md`; do not duplicate every internal PR.

## Preflight evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| Swarm `Shiplog Rust Small Result` | pending | `<run>` |
| `cargo xtask ci-small` | pending | `<run or local receipt>` |
| Repository contract report | pending | `<receipt>` |
| Deterministic promotion dry-run | pending | `<receipt hashes / overlay>` |
| Source promotion exact-head CI | pending | `<run>` |
| Promotion `--verify-only` | pending | `<receipt>` |
| Source post-merge CI | pending | `<run>` |
| Package version audit | pending | `<output>` |
| Package boundary audit | pending | `<output>` |
| Package proof | pending | `<run>` |
| Allowlisted publish dry-run | pending | `<run>` |
| Release-hold guard | pending | `<output>` |

## Exact-tag artifact evidence

| Gate | Result | Evidence |
| --- | --- | --- |
| Release Preflight | pending | `<job>` |
| Linux x86-64 build | pending | `<job/artifact>` |
| macOS Intel build | pending | `<job/artifact>` |
| macOS Apple Silicon build | pending | `<job/artifact>` |
| Windows x86-64 build | pending | `<job/artifact>` |
| Staged candidate manifest and artifact digests | pending | `<artifact/manifest>` |
| Checksums and draft assets | pending | `<job>` |
| Release Validation | pending | `<job>` |
| Linux first-use acceptance | pending | `<job>` |
| macOS Intel first-use acceptance | pending | `<job>` |
| macOS Apple Silicon first-use acceptance | pending | `<job>` |
| Windows first-use acceptance | pending | `<job>` |
| Release-mode integration tests | pending | `<job>` |
| Terminal `Release Candidate Ready` aggregate | pending | `<job>` |

## Publication gates

- [ ] Exact immutable tag SHA is confirmed.
- [ ] All applicable exact-tag jobs consume one staged candidate set and are green.
- [ ] Terminal `Release Candidate Ready` is green for that exact tag and manifest.
- [ ] GitHub release remains draft before publication approval.
- [ ] crates.io package version and metadata are final.
- [ ] Public asset names and checksums are final.
- [ ] Homebrew and Scoop update inputs are final.
- [ ] Known limitations and deferred work are explicit.

## Blockers and exceptions

List every unresolved item. A skipped or unavailable check must be stated as
not verified rather than silently converted to a pass.

## Release readiness decision

`READY` or `BLOCKED`, with the exact evidence boundary and owner approval still
required for tag push/publication.
