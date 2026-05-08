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
| `shiplog-coverage` | `762841b` | 31 | 26 | 0 | 5 | clean baseline |
| `shiplog-ids` | `e6166e5` | 8 | 5 | 0 | 3 | clean baseline |
| `shiplog-ports` | `74d095d` | 0 | 0 | 0 | 0 | no mutation targets |
| `shiplog-schema` | `77dc752` | 33 | 27 | 0 | 6 | clean baseline |

The local PowerShell receipts used:

```powershell
New-Item -ItemType Directory -Force -Path target\mutants | Out-Null
cargo mutants -p shiplog-coverage --timeout 600 --copy-target=false --output target/mutants/shiplog-coverage-baseline
cargo mutants -p shiplog-ids --timeout 600 --copy-target=false --output target/mutants/shiplog-ids-baseline
cargo mutants -p shiplog-ports --timeout 600 --copy-target=false --output target/mutants/shiplog-ports-baseline
cargo mutants -p shiplog-schema --timeout 600 --copy-target=false --output target/mutants/shiplog-schema-baseline
```

`cargo-mutants` reported:

```text
shiplog-coverage:
31 mutants tested in 2m: 26 caught, 5 unviable

shiplog-ids:
8 mutants tested in 50s: 5 caught, 3 unviable

shiplog-ports:
Found 0 mutants to test
WARN No mutants found under the active filters

shiplog-schema:
33 mutants tested in 4m: 27 caught, 6 unviable
```

The generated `missed.txt` files were empty, so there were no surviving mutants
for these crates in the baseline runs.

`shiplog-ports` is a trait contract crate. The current cargo-mutants scan found
no mutable implementation targets; its behavior remains covered by trait-object,
error-path, and composition tests rather than mutation survivor counts.

## Next baseline candidates

Record the remaining Tier 1 crates one at a time before enforcing mutation
thresholds:

- `shiplog-redact`
- `shiplog-bundle`

Keep each baseline as evidence, not a PR gate, until repeated scheduled runs
show stable timings and stable survivor counts.

## Claim boundary

Mutation results are test-strength evidence for the mutated code only. A clean
mutation baseline for one crate does not prove source adapter completeness,
packet quality, redaction safety, or release readiness.
