# shiplog 0.5.0 Release Status

**Date:** 2026-05-10  
**Status:** Ready for crates.io publish  

## Merge & Build Status

| Step | Status | Details |
|------|--------|---------|
| PR #157 merge | ✅ | Squash merged to `main` (SHA `97347c89df91c1c11bb2169be90428a8c68c3a38`) |
| Version bump | ✅ | 0.4.0 → 0.5.0 across 24 crates |
| CHANGELOG | ✅ | `[0.5.0] - 2026-05-10` finalized |
| Release docs | ✅ | `docs/release/0.5.0-readiness.md` + `RELEASE_HANDOFF_0.5.0.md` |
| package-proof.sh | ✅ | All gates cleared (tests, audit, deny, fmt, clippy) |
| Dry-run publish | ✅ | `cargo publish -p shiplog-ids --dry-run` PASS |
| Release build | ✅ | `target/release/shiplog 0.5.0` compiles |
| Binary smoke | ✅ | --version, --help, intake --help verified |
| Git tag | ✅ | `v0.5.0` created locally on merge commit |
| Git push | ⚠️  | Blocked by proxy (HTTP 403); does not block crates.io publish |

## Pre-Publish Checklist

- [x] Main is at v0.5.0 commit
- [x] CI gates (package-proof.sh) cleared
- [x] Binary built and smoke-tested
- [x] Dry-run verified
- [x] Tag created
- [ ] crates.io publish sequence (awaiting credentials)

## Publish Order (Foundation → Adapters → Engine → CLI)

```
shiplog-ids             (foundation, no deps)
shiplog-schema
shiplog-ports
shiplog-coverage        (trust surface)
shiplog-cache
shiplog-redact
shiplog-bundle
shiplog-workstreams
shiplog-render-md
shiplog-render-json
shiplog-cluster-llm     (adapters)
shiplog-ingest-github
shiplog-ingest-git
shiplog-ingest-json
shiplog-ingest-manual
shiplog-ingest-gitlab
shiplog-ingest-jira
shiplog-ingest-linear
shiplog-team
shiplog-merge
shiplog-engine          (orchestration)
shiplog                 (CLI)
```

All verified dry-run ready. Publish requires `CARGO_REGISTRY_TOKEN`.
