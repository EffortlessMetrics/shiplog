# Code Review Invariants

This document captures the invariants that code reviews (human and automated) must maintain.

## Evidence Standards

### Evidence Categories

Reviews must separate evidence into three categories:

1. **Observed**
   - Results from running the code (tests, linters, type-checkers)
   - Direct inspection (code walkthrough, diff analysis)
   - Reproducible in the reviewer's environment

2. **Reported**
   - Output from external tools (CI/CD, analyzers, scanners)
   - Claims from the author in PR body/commit messages
   - Facts from repository metadata (git history, issue links)

3. **Not verified**
   - Inferred or untested claims
   - Author claims not independently confirmed
   - Changes that require runtime behavior not visible in tests

Example finding with proper evidence separation:

```
Security concern: Unchecked user input in request handler

Observed:
- Handler accepts query parameter without validation
- No test coverage for malicious input
- Type signature allows arbitrary strings

Reported:
- clippy reports no security issues
- Integration tests pass with normal input

Not verified:
- Whether this parameter is exposed to untrusted users
- Risk level without full application context
```

## Review Integrity

### No Naked LGTM

Approvals must include:
- At least one validation signal (test pass, design review, security check)
- Explicit reasoning tying the signal to the approval
- Known gaps or residual risks

❌ Bad: "LGTM"
✅ Good: "All tests pass and integration with downstream service is verified. One edge case (timeout retry) remains untested but is guarded by existing timeout handler."

### Actionable Findings

Every comment must be a repair packet:

```
[P1] Pattern: unchecked result in error path

Failure mode:
If this operation fails, the error is silently dropped.

Why here:
The ? operator requires Result type, which this function returns.

Fix direction:
Add logging or explicit error handling before the ?
  // Before
  some_operation()?;
  
  // After
  some_operation().context("operation failed")?;

Validation:
New tests should exercise the error path and verify logging output.

Confidence:
Medium (depends on whether this path is actually reachable in practice)
```

### Comment Scope

- **No extra @mentions**: Direct mentions only to PR author and reviewing team
- **No passive-aggressive tone**: Findings are objective; fix direction is respectful
- **No meta-comments**: "I noticed you...", "You forgot...", "Why did you...?" → direct to finding

❌ Bad: "@alice I can't believe you didn't test this!"
✅ Good: "[P1] Untested code path. Tests should exercise error handling in connection retry."

## Clean Reviews

When no actionable findings exist, the review must include an inspection record.

### Minimum Elements

1. **Inspected surfaces**: What parts of the code were examined
2. **Checks performed**: What automated or manual checks ran
3. **Why no comments**: Explicit statement of why findings do not warrant comments
4. **Residual risk**: Known gaps or uncovered scenarios
5. **Validation signals**: Evidence split into Observed / Reported / Not verified

### Template

```
No actionable findings emitted.

Inspected surfaces:
- [Component/file/module list]
- [Pattern: search term or specific code area]

Checks performed:
- [Tool name]: [result]
- [Test suite]: [coverage level]
- [Manual inspection]: [focus area]

Why no comments:
[Explicit statement of why the code is sound despite gaps]

Residual risk:
- [Known untested scenario]
- [Integration boundary not verified]

Validation signal:
  Observed:
    - [Test evidence]
    - [Linter evidence]
  Reported:
    - [CI output]
    - [Tool output]
  Not verified:
    - [Runtime behavior claim]
    - [Assumption about caller]
```

## PR-Body Validation Claims

Claims made in PR descriptions or commit messages are **not independently verified** by reviewers.

Instead:
- Reviewers verify claims against code, tests, and CI output
- If a claim cannot be verified, it moves to "Not verified"
- Critical claims require explicit test coverage

Example PR body claim:

```
## This PR

- Fixes transaction isolation under high contention
- Adds backpressure for pending writes
- Backward compatible with v1.x clients
```

Reviewer response:

```
✅ Observed:
  - Tests for high-contention scenario pass
  - Backpressure metrics present in code
  
✅ Reported:
  - CI checks green; no regressions in compatibility tests
  
❓ Not verified:
  - Real-world performance improvement (requires load test)
  - Whether v1.x clients can actually handle backpressure gracefully
```

## No Comment Cap

All findings are reported. Reviewers do not suppress findings to keep review length reasonable.

Approaches to manage review volume:

1. **Batch by severity**: P0 (critical) first, then P1, then P2
2. **Group related findings**: Bundle similar pattern occurrences
3. **Distinguish scope**: Separate findings into "must fix" vs "consider for next cycle"

Do not:
- Limit findings to a fixed number (e.g., "top 5 issues only")
- Suppress findings because the review is already long
- Bundle unrelated issues to reduce comment count

## Review Timeline

- **Draft review**: Can be published immediately; supports iterative feedback
- **Final review**: Published when comprehensive examination is complete
- **Update signal**: If review was draft, mark update when examination completes

## References

- [Droid Automation](../../AGENTS.md) — Droid review configuration
- [Review Guidelines Skill](.factory/skills/review-guidelines/SKILL.md) — Droid behavior
- [Droid Review Rules](.factory/rules/droid-review.md) — Technical requirements
