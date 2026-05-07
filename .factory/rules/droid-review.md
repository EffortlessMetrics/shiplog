# Droid Review Rules

Rules governing Droid code review automation for shiplog.

## Action Safety

- **Safe action ref**: `EffortlessMetrics/droid-action-safe@01e76b659e4b1e5f23feedc8cfabf8dc14c7485f`
- **Based on**: Factory-AI/droid-action v5
- **Change**: Raw debug artifact upload disabled
- **Never**: Direct use of `Factory-AI/droid-action` for BYOK workflows

## Credential Handling

### MiniMax API Key

- Source: `MINIMAX_API_KEY` GitHub secret
- Transport: Quoted heredoc in `~/.factory/settings.local.json`
- Literal format: `"${MINIMAX_API_KEY}"` (not interpolated at step level)
- Scope: Workflows only; never logged or committed
- Rotation: Outside Droid workflows; use GitHub org secret management

### Factory API Key

- Source: `FACTORY_API_KEY` GitHub secret
- Required for all Droid runs
- Must be present with `MINIMAX_API_KEY`; runs skip if either secret is unavailable

## Workflow Guards

### Same-Repo Guard (Auto Review)

Prevents fork PRs from accessing secrets:

```yaml
github.event.pull_request.head.repo.full_name == github.repository
```

### Trusted Actor Guard (Manual @droid)

Prevents arbitrary public comments from triggering secret-backed jobs:

```yaml
contains(fromJSON('["OWNER","MEMBER","COLLABORATOR"]'), github.event.comment.author_association)
```

### Skip Tag

Allow PR authors to bypass auto review:

```yaml
!contains(github.event.pull_request.title, '[skip-review]')
```

## Review Configuration

- **Model**: `custom:MiniMax-M2.7-0`
- **Depth**: `shallow`
- **Output**: `show_full_output: false`
- **Artifacts**: `upload_debug_artifacts: false`
- **Suggestions**: `include_suggestions: true`
- **Temperature**: 1.0
- **Max tokens**: 64000

## Permissions

### Auto Review (`droid-review.yml`)

```yaml
contents: write       # Required: publish review
pull-requests: write  # Required: create/update review
issues: write         # Required: create issues for findings
id-token: write       # Required: OIDC token generation
actions: read         # Required: inspect workflow context
```

### Manual Tag (`droid.yml`)

```yaml
contents: read        # Manual requests are read-only; no auto-publish
pull-requests: write  # Required: create/update review
issues: write         # Required: create issues for findings
id-token: write       # Required: OIDC token generation
actions: read         # Required: inspect workflow context
```

### Security Scan (`droid-security-scan.yml`)

```yaml
contents: write       # Required: publish scan report
pull-requests: write  # Required: create/update review
issues: write         # Required: create issues for findings
id-token: write       # Required: OIDC token generation
actions: read         # Required: inspect workflow context
```

## Expected Behavior

### Auto Review

- Triggers on: `pull_request` (opened, synchronize, ready_for_review, reopened)
- Skips: Drafts, fork PRs, [skip-review] tagged PRs
- Publishes: Draft review with findings or clean inspection record
- Artifacts: None (raw debug artifacts disabled)

### Manual @droid

- Trigger: `@droid` in comment/issue/PR from trusted actor
- Publishes: Regular or draft review depending on Factory behavior
- Artifacts: None
- No secrets exposed to fork PRs

### Scheduled Security Scan

- Trigger: Monday 08:00 UTC or manual dispatch
- Scope: Full repository security analysis
- Publishes: Issues or pull request with findings
- Artifacts: None

## Validation

Every merged Droid workflow change must pass:

1. **Syntax check**: Valid YAML
2. **Ref validation**: No direct `Factory-AI/droid-action` use
3. **Guard validation**: Same-repo + trusted-actor guards present
4. **Artifact validation**: `upload_debug_artifacts: false` enforced
5. **Smoke test**: At least one same-repo draft PR review succeeds

## Non-Goals

- No permission reduction without explicit safety review
- No `pull_request_target` event (fork PR secret exposure risk)
- No custom model unless explicitly approved and security-reviewed
- No fork PR secret execution
- No raw `.factory/**` or `droid-prompts/**` artifact upload
