# Review Guidelines Skill

This skill configures Droid code review behavior for shiplog.

## Key Constraints

1. **No mutable refs**: All action refs must be pinned to commit SHAs
2. **Safe action only**: Use `EffortlessMetrics/droid-action-safe` for all Droid workflows
3. **No raw debug artifacts**: `upload_debug_artifacts` must be `false`
4. **BYOK credentials**: MiniMax API key provided via `~/.factory/settings.local.json`
5. **Shallow review**: Use `review_depth: shallow` for all reviews
6. **MiniMax model**: Always use `custom:MiniMax-M2.7-0`

## Review Model Selection

- **Default model**: `custom:MiniMax-M2.7-0`
- **Security model**: `custom:MiniMax-M2.7-0`
- **Temperature**: 1.0
- **Max output tokens**: 64000

## Droid Workflow Gates

### Auto Review Guard

Same-repo origin only. Block fork PRs from accessing FACTORY_API_KEY or MINIMAX_API_KEY:

```yaml
if: |
  github.event.pull_request.head.repo.full_name == github.repository &&
  !contains(github.event.pull_request.title, '[skip-review]')
```

### Manual Tag Guard

Trusted actors only (OWNER, MEMBER, COLLABORATOR):

```yaml
contains(fromJSON('["OWNER","MEMBER","COLLABORATOR"]'), github.event.comment.author_association)
```

## Review Depth and Output

- **review_depth**: `shallow` (no deep analysis; focus on clear wins)
- **show_full_output**: `false` (suppress verbose logging)
- **include_suggestions**: `true` (include minor improvement suggestions)
- **security_severity_threshold**: `high` (auto-review); `medium` (scheduled security scan)
- **security_block_on_critical**: `true`
- **security_block_on_high**: `false`

## Clean Review Evidence

Every clean review must include:

1. **Inspected surfaces**: Files, components, or modules covered
2. **Checks performed**: Automated checks, linting, type-checking results
3. **Why no comments**: Explanation of why findings do not require review comments
4. **Residual risk**: Known gaps or uncovered areas
5. **Validation signals**:
   - Observed: Evidence from running tests or code inspection
   - Reported: Tool/CI output confirming no issues
   - Not verified: Claims not independently confirmed

Example:

```
No actionable findings emitted.

Inspected surfaces:
- Rust crate changes in shiplog-engine
- Workflow configuration updates

Checks performed:
- cargo clippy --all-targets --all-features
- cargo test --workspace
- YAML validation

Why no comments:
All logic changes pass tests and clippy checks.
Workflow changes follow EffortlessMetrics patterns.

Residual risk:
Integration with external services (GitHub API, MiniMax) tested at boundary only.

Validation signal:
  Observed:
    - All tests pass
    - No clippy warnings
  Reported:
    - CI checks green
  Not verified:
    - MiniMax API behavior under load
```

## References

See also:
- [Droid Review Rules](../rules/droid-review.md)
- [Review Invariants](../../docs/agent-context/review-invariants.md)
- [Smoke Tests](../../docs/agent-context/droid-smoke-tests.md)
