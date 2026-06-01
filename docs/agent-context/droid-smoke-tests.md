# Droid Smoke Test Checklist

Validation checklist for Droid workflow changes and rollout.

## Static Validation

Run these checks locally before pushing workflow changes:

```bash
# YAML syntax
cargo xtask check-workflows 2>/dev/null || echo "Skipped: no cargo xtask"

# Forbidden workflow patterns
rg "uses: Factory-AI/droid-action|droid-action@main|droid-action@v[0-9]+|upload_debug_artifacts: true|show_full_output: true" \
  .github/workflows/droid*.yml 2>/dev/null && echo "ERROR: Found forbidden Droid workflow pattern" || true

# Checkout action pinning for Droid workflows
rg -P 'uses: actions/checkout@(?![0-9a-f]{40}(\s|$|#))[^\s#]+' \
  .github/workflows/droid*.yml 2>/dev/null && echo "ERROR: Unpinned Droid checkout" || true

# Safe action ref
rg 'uses: EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec' \
  .github/workflows/droid*.yml | wc -l
# Expected: 3 (one per workflow file)
```

Expected results:

- ✅ No direct `Factory-AI/droid-action` workflow use
- ✅ No mutable Droid action `@main` or `@v*` refs
- ✅ No `upload_debug_artifacts: true`
- ✅ No `show_full_output: true`
- ✅ All safe action refs pinned correctly
- ✅ Droid checkout actions pinned to commit SHA
- ✅ 3 instances of safe action ref (droid-review, droid, droid-security-scan)

## Live Smoke Validation

For each Droid workflow change, validate with a same-repo non-draft PR:

### 1. Auto Review Trigger

```
1. Create a local branch: git checkout -b test-droid-<date>
2. Make a trivial change (comment, whitespace, test tweak)
3. Commit: git commit -m "test: droid smoke test"
4. Push: git push -u origin test-droid-<date>
5. Open a non-draft PR via GitHub UI
6. Wait 2–3 minutes for Droid Auto Review to trigger
```

Expected: Droid Auto Review job starts and completes within 5–10 minutes.

### 2. Auto Review Model Verification

```
1. Open the Droid Auto Review workflow run
2. Click the "Run Droid Auto Review with MiniMax M3 BYOK" step
3. Check logs for:
   - "Processing with model: custom:MiniMax-M3-0"
   - No errors or timeouts
   - Review published (review comment appears on PR)
```

Expected: Review uses MiniMax M3 model; no API errors.

### 3. Artifact Validation

```
1. Open the workflow run's Artifacts section
2. Search for artifact named "droid-review-debug-*"
```

Expected: **No artifact** named `droid-review-debug-<run_id>` is present.

If sanitized artifacts were explicitly enabled for diagnostics:
- Expected artifact name: `droid-review-debug-sanitized-<run_id>`
- Normal rollout: This should never appear

### 4. Review Quality

```
1. View the review comment on the test PR
2. Check for inspection record if no findings:
   - Inspected surfaces section
   - Checks performed section
   - Why no comments section
   - Residual risk section
   - Validation signal section with Observed/Reported/Not verified split
3. If there are findings, check for:
   - [P0|P1|P2] priority label
   - Failure mode section
   - Why here section
   - Fix direction with code examples
   - Validation section
   - Confidence assessment
```

Expected: Clean review or findings follow the proper format.

### 5. Manual @droid Review (Trusted Actor)

```
1. As the PR author or repo member, comment on the PR:
   @droid review
2. Wait 2–3 minutes for job to trigger
3. Check workflow run for same quality checks as #4 above
```

Expected: Manual review runs; uses same MiniMax model; follows format.

### 6. Manual @droid Security (Trusted Actor)

```
1. As the PR author or repo member, comment on the PR:
   @droid security
2. Wait 2–3 minutes for job to trigger
3. Check logs for security-focused analysis
```

Expected: Security analysis runs; model uses `custom:MiniMax-M3-0`.

### 7. Fork PR Secret Containment

```
1. Create a fork of this repository
2. From the fork, open a PR against the main repo
3. In the PR, comment: @droid review
4. Wait 2–3 minutes
```

Expected: Droid does NOT trigger (fork PR protection); if it does, verify that no secrets are exposed in logs.

### 8. Scheduled Security Scan (if enabled)

```
1. Open GitHub UI → Actions → Droid Security Scan
2. Click "Run workflow" → select branch → confirm
3. Wait 5–10 minutes for scan to complete
4. Check the run output for:
   - Model: custom:MiniMax-M3-0
   - Severity threshold: medium
   - Block on critical: true
   - No debug artifacts uploaded
```

Expected: Scan completes; no artifacts uploaded.

### 9. MiniMax Usage Visibility

```
1. Log in to MiniMax dashboard (if available)
2. Check API usage for the MINIMAX_API_KEY
3. Verify that calls appeared during smoke test runs
```

Expected: At least 2–4 API calls from test runs are visible.

### 10. Cleanup

```
1. Close the test PR
2. Delete the test branch: git push origin --delete test-droid-<date>
3. Verify workflow runs completed without errors
```

## Validation Failure Troubleshooting

### Droid Auto Review does not trigger

- Check: Is the PR non-draft?
- Check: Does the PR title contain `[skip-review]`?
- Check: Is `FACTORY_API_KEY` configured as a GitHub secret?
- Check: Is the PR from the same repository (not a fork)?
- Action: Wait longer (up to 10 minutes); GitHub Actions can be slow

### Manual @droid does not trigger

- Check: Did you use the exact syntax `@droid review` or `@droid security`?
- Check: Are you the PR author or a repo OWNER/MEMBER/COLLABORATOR?
- Check: Is `FACTORY_API_KEY` configured?
- Action: Try a new comment; sometimes first trigger is slow

### No MiniMax model in logs

- Check: Is the safe action ref correct?
  `EffortlessMetrics/droid-action-safe@7c1377ccbacddc95560d1570547a5baa51de01ec`
- Check: Is `MINIMAX_API_KEY` configured as a GitHub secret?
- Check: Is the MiniMax BYOK step running before the action?
- Action: Check workflow file for MiniMax config step

### Artifact uploaded (should not be present)

- Check: Is `upload_debug_artifacts: false` set in all action steps?
- Check: Is the action ref the safe variant?
- Action: Review workflow definition; may need to update action ref

### Fork PR triggered secrets

- Check: Same-repo guard present in auto-review?
  `github.event.pull_request.head.repo.full_name == github.repository`
- Check: Trusted-actor guard present in manual tag?
  `contains(fromJSON('["OWNER","MEMBER","COLLABORATOR"]'), ...)`
- Action: Review and strengthen guards

## Sign-Off Criteria

All smoke tests must pass before:

1. Merging Droid workflow changes to main
2. Rolling out Droid to other repositories
3. Running production security scans

Example sign-off:

```
Droid smoke test results (date: 2026-05-07, branch: claude/migrate-factory-pattern-x4v9i):

✅ Static validation: No forbidden patterns
✅ Auto review trigger: Job ran within 5 minutes
✅ Model verification: custom:MiniMax-M3-0 confirmed
✅ No raw artifacts: Only review comment published
✅ Review format: Inspection record present
✅ Manual @droid review: Ran successfully
✅ Manual @droid security: Ran successfully
✅ Fork PR protection: No secret exposure
✅ MiniMax visibility: 4 API calls observed
✅ Cleanup: Test branch removed

Status: Ready for rollout
```
