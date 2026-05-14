# Mutation Testing

Mutation testing is behavioral test-strength evidence.

It answers:

> Would the tests catch small behavior changes in trusted code?

It does not answer:

- whether source adapters are complete against provider APIs,
- whether packet language is review-ready,
- whether redaction is appropriate for a specific audience,
- whether coverage manifests are complete for a real user,
- whether release packaging is proven.

Those are separate proof lanes.

## Workflow posture

The Mutation Testing workflow runs on `workflow_dispatch` and a weekly schedule.
Pull request events create a skipped check. The workflow is intentionally
advisory until crate-level baselines are established and reviewed.

## Current baseline

Recorded Tier 1 baselines:

| Crate | Commit | Mutants | Caught | Survived | Unviable | Result |
| ----- | ------ | ------: | -----: | -------: | -------: | ------ |
| `shiplog::coverage` | `762841b` | 31 | 26 | 0 | 5 | historical baseline from the former `shiplog-coverage` crate |
| `shiplog-ids` | `e6166e5` | 8 | 5 | 0 | 3 | clean baseline |
| `shiplog-ports` | `74d095d` | 0 | 0 | 0 | 0 | no mutation targets |
| `shiplog-schema` | `77dc752` | 33 | 27 | 0 | 6 | clean baseline |
| `shiplog::redact` | `812c45b` + policy cleanup | 35 | 26 | 0 | 9 | historical baseline from the former `shiplog-redact` crate |
| `shiplog::bundle` | `f18b23d` + zip/hash cleanup | 22 | 21 | 0 | 1 | historical baseline from the former `shiplog-bundle` crate |

The local PowerShell receipts used:

```powershell
New-Item -ItemType Directory -Force -Path target\mutants | Out-Null
cargo mutants -p shiplog --timeout 600 --copy-target=false --output target/mutants/shiplog-coverage-baseline
cargo mutants -p shiplog-ids --timeout 600 --copy-target=false --output target/mutants/shiplog-ids-baseline
cargo mutants -p shiplog-ports --timeout 600 --copy-target=false --output target/mutants/shiplog-ports-baseline
cargo mutants -p shiplog-schema --timeout 600 --copy-target=false --output target/mutants/shiplog-schema-baseline
cargo mutants -p shiplog-redact --timeout 600 --copy-target=false --output target/mutants/shiplog-redact-baseline-public-source
cargo mutants -p shiplog-bundle --timeout 600 --copy-target=false --output target/mutants/shiplog-bundle-baseline-fixed
```

`cargo-mutants` reported:

```text
shiplog::coverage:
31 mutants tested in 2m: 26 caught, 5 unviable

shiplog-ids:
8 mutants tested in 50s: 5 caught, 3 unviable

shiplog-ports:
Found 0 mutants to test
WARN No mutants found under the active filters

shiplog-schema:
33 mutants tested in 4m: 27 caught, 6 unviable

shiplog::redact:
35 mutants tested in 2m: 26 caught, 9 unviable

shiplog::bundle:
22 mutants tested in 2m: 21 caught, 1 unviable
```

The generated `missed.txt` files for `shiplog::coverage`, `shiplog-ids`,
`shiplog-schema`, `shiplog::redact`, and `shiplog::bundle` were empty, so there
were no surviving mutants for these crates in the baseline runs.

The first `shiplog::redact` scan found two equivalent survivors in batch-level
`Internal` fast paths. The policy cleanup in this baseline removes those
duplicate fast paths so profile semantics live at the single-event and
single-workstream policy boundary; the clean rerun is the recorded baseline.
The current inlined module is covered by the `shiplog` package mutation target.

The first `shiplog-bundle` scan found a timeout in the manual file-read loop
used for checksum hashing. The baseline cleanup switched hashing to
`read_to_end` and fixed a bundle safety gap where a zip output written inside
the run directory could be included in the archive being written. The clean
rerun is the recorded baseline.

`shiplog-ports` is a trait contract crate. The current cargo-mutants scan found
no mutable implementation targets; its behavior remains covered by trait-object,
error-path, and composition tests rather than mutation survivor counts.

## Next baseline posture

The current Tier 1 baseline set is recorded. Keep these baselines as evidence,
not a PR gate, until repeated scheduled runs show stable timings and stable
survivor counts.

## Claim boundary

Mutation results are test-strength evidence for the mutated code only. A clean
mutation baseline for one crate does not prove source adapter completeness,
packet quality, redaction safety, or release readiness.
