# shiplog.toml Reference

`shiplog.toml` is the repeat-use configuration for `shiplog collect multi` and
`shiplog intake`. It records default output, date-window, named review periods,
profile, and source settings. Secrets do not belong in this file; keep provider
tokens in environment variables.

Use these commands before collecting:

```bash
shiplog config validate --config shiplog.toml
shiplog config explain --config shiplog.toml
shiplog doctor --config shiplog.toml
shiplog doctor --config shiplog.toml --repair-plan
```

`config validate` checks the file shape, configured date window, enabled source
settings, enum values, and local paths without requiring provider tokens.
`config explain` prints the resolved defaults and enabled sources. `doctor`
adds readiness checks such as token presence, identity discovery, output
writability, and redaction-key safety. `doctor --repair-plan` is read-only: it
classifies setup issues and prints source-by-source repair commands without
collecting data or writing run artifacts.

For Jira and Linear identity values, run:

```bash
shiplog identify jira --instance company.atlassian.net --auth-user you@example.com
shiplog identify linear
```

The Jira helper prints the authenticated Atlassian account ID for `user`. The
Linear helper prints the authenticated Linear user ID for `user_id`.

## Versioning

New configs include explicit version metadata:

```toml
[shiplog]
config_version = 1
```

Missing version metadata is accepted as implicit v1 for compatibility with
older configs. Unsupported future versions fail validation, doctor, and
collection before output files are written.

To add the current version metadata to an older config:

```bash
shiplog config migrate --config shiplog.toml --dry-run
shiplog config migrate --config shiplog.toml
```

Migration only inserts `[shiplog] config_version = 1`; it does not reorder or
rewrite source settings.

## Paths

Relative paths are resolved from the directory containing `shiplog.toml`.
`defaults.out` defaults to `./out`. Source API cache directories default to
`<defaults.out>/.cache` unless a source sets `cache_dir` or `no_cache = true`.

## Environment Variables

| Variable | Used by | Notes |
|----------|---------|-------|
| `GITHUB_TOKEN` | GitHub | Required by `doctor` and collection when GitHub is enabled. |
| `GITLAB_TOKEN` | GitLab | Required by `doctor` and collection when GitLab is enabled. |
| `JIRA_TOKEN` | Jira | Required by `doctor` and collection when Jira is enabled. |
| `LINEAR_API_KEY` | Linear | Required by `doctor` and collection when Linear is enabled. |
| `SHIPLOG_REDACT_KEY` | Redaction | Default key env var for manager/public packets and bundles. |
| `JIRA_AUTH_USER` | Jira example | Only used if `sources.jira.auth_user_env = "JIRA_AUTH_USER"`. |

You can change the redaction key env var with `[redaction] key_env`.
Do not put token values in `shiplog.toml`.

## Defaults

| Field | Default | Notes |
|-------|---------|-------|
| `defaults.out` | `./out` | Output root used by multi-source collection and cache defaults. |
| `defaults.window` | `last-6-months` | Used by `collect multi` unless CLI date flags are provided. |
| `defaults.profile` | `internal` | One of `internal`, `manager`, or `public`. |
| `defaults.include_reviews` | `false` | Source-specific `include_reviews` overrides this. |
| `user.label` | unset | Human label used by rendering and manual-source fallback. |
| `redaction.key_env` | `SHIPLOG_REDACT_KEY` | Env var used for share-profile redaction keys. |

Supported configured windows:

```toml
[defaults]
window = "last-6-months"
window = "last-quarter"
window = "year:2025"
window = "year=2025"
window = "2025"
```

CLI date flags such as `--last-6-months`, `--last-quarter`, `--year`,
`--since`, and `--until` override `defaults.window`.

## Named Periods

Named periods make review windows repeatable without retyping dates. They are
available to config-driven commands:

```bash
shiplog intake --period review-cycle
shiplog collect multi --period 2026-H1
shiplog review --period 2026-H1
shiplog runs compare --from-period 2025-H2 --to-period 2026-H1
shiplog periods list
shiplog periods explain review-cycle
```

Define a period with either explicit dates:

```toml
[periods."2026-H1"]
since = "2026-01-01"
until = "2026-07-01"
```

or a supported preset:

```toml
[periods."review-cycle"]
preset = "last-6-months"
```

Supported period presets match `defaults.window`: `last-6-months`,
`last-quarter`, `year:YYYY`, `year=YYYY`, or a bare four-digit year. A period
must use either `preset` or `since`/`until`, not both. CLI date flags still win
over `--period` when both are supplied.

`config explain` prints each period with resolved concrete dates.
`shiplog periods list` and `shiplog periods explain <name>` show the period
definition, resolved window, latest matching run when one exists, and suggested
intake/review commands. Intake reports record the selected period name and
resolved window so later reruns and review checks can explain which review
cycle produced the packet. `shiplog runs compare --from-period <name>
--to-period <name>` compares the latest matching run for each period without
rewriting packets or coverage artifacts.

## Source Sections

Each source section is optional. A source participates in `collect multi` only
when its section exists and `enabled = true`.

### GitHub

```toml
[sources.github]
enabled = true
user = "your-github-login"
me = false
mode = "merged"
include_reviews = true
no_details = false
throttle_ms = 0
api_base = "https://api.github.com"
cache_dir = "./out/.cache"
no_cache = false
```

Required when enabled: either `user` or `me = true`, but not both.

| Field | Notes |
|-------|-------|
| `user` | GitHub login to query. |
| `me` | Infer the authenticated user via `GITHUB_TOKEN`. |
| `mode` | `merged` or `created`; defaults to `merged`. |
| `include_reviews` | Overrides `defaults.include_reviews`. |
| `no_details` | Skip detail fetches when true. |
| `throttle_ms` | Delay between provider requests. |
| `api_base` | API base URL; defaults to `https://api.github.com`. |
| `cache_dir` | Source cache directory. |
| `no_cache` | Disable this source cache when true. |

### GitLab

```toml
[sources.gitlab]
enabled = true
user = "your-gitlab-username"
me = false
instance = "gitlab.com"
state = "merged"
include_reviews = true
throttle_ms = 0
cache_dir = "./out/.cache"
no_cache = false
```

Required when enabled: either `user` or `me = true`, but not both.

| Field | Notes |
|-------|-------|
| `user` | GitLab username to query. |
| `me` | Infer the authenticated user via `GITLAB_TOKEN`. |
| `instance` | Hostname such as `gitlab.com` or a self-hosted GitLab instance. |
| `state` | `opened`, `merged`, `closed`, or `all`; defaults to `merged`. |
| `include_reviews` | Overrides `defaults.include_reviews`. |
| `throttle_ms` | Delay between provider requests. |
| `cache_dir` | Source cache directory. |
| `no_cache` | Disable this source cache when true. |

### Jira

```toml
[sources.jira]
enabled = true
user = "712020:your-account-id"
auth_user_env = "JIRA_AUTH_USER"
instance = "company.atlassian.net"
status = "done"
throttle_ms = 0
cache_dir = "./out/.cache"
no_cache = false
```

Required when enabled: `user` and `instance`.

| Field | Notes |
|-------|-------|
| `user` | Jira assignee JQL value. This may be an account ID, email, or value accepted by your Jira instance. |
| `auth_user` | Optional Basic Auth username/email when different from `user`. |
| `auth_user_env` | Env var containing the auth user, for example `JIRA_AUTH_USER`. |
| `instance` | Jira hostname such as `company.atlassian.net`. |
| `status` | `open`, `in_progress`, `done`, `closed`, or `all`; defaults to `done`. |
| `throttle_ms` | Delay between provider requests. |
| `cache_dir` | Source cache directory. |
| `no_cache` | Disable this source cache when true. |

If neither `auth_user` nor `auth_user_env` is set, Jira authentication defaults
to the assignee value in `user`.

Use `shiplog identify jira --instance <host> --auth-user <email>` to print the
authenticated account ID when you do not know the right `user` value.

### Linear

```toml
[sources.linear]
enabled = true
user_id = "your-linear-user-id"
status = "done"
project = ""
throttle_ms = 0
cache_dir = "./out/.cache"
no_cache = false
```

Required when enabled: `user_id`.

| Field | Notes |
|-------|-------|
| `user_id` | Linear user ID to query. |
| `status` | `backlog`, `todo`, `in_progress`, `done`, `cancelled`, or `all`; defaults to `done`. |
| `project` | Optional project filter. |
| `throttle_ms` | Delay between provider requests. |
| `cache_dir` | Source cache directory. |
| `no_cache` | Disable this source cache when true. |

Use `shiplog identify linear` to print the authenticated user ID when you do
not know the right `user_id` value.

### Local Git

```toml
[sources.git]
enabled = true
repo = "."
author = "you@example.com"
include_merges = false
```

Required when enabled: `repo`.

| Field | Notes |
|-------|-------|
| `repo` | Path to a local git repository. |
| `author` | Optional author filter. |
| `include_merges` | Include merge commits when true. |

### JSON

```toml
[sources.json]
enabled = true
events = "./ledger.events.jsonl"
coverage = "./coverage.manifest.json"
```

Required when enabled: `events` and `coverage`, both pointing to existing
files.

### Manual

```toml
[sources.manual]
enabled = true
events = "./manual_events.yaml"
user = "Your Name"
```

Required when enabled: `events`, pointing to an existing manual events file.
`user` defaults to `[user].label` when present.

## Redaction Safety

Internal packets can render without a redaction key. Manager and public packets
and bundles fail closed unless the configured redaction key env var is present
or the CLI command passes `--redact-key`.

```toml
[defaults]
profile = "manager"

[redaction]
key_env = "SHIPLOG_REDACT_KEY"
```

For share profiles, `config validate` can still pass without the key because it
does not inspect secrets. Use `shiplog doctor` before collection or rendering
to catch missing redaction keys.

## Examples

Copy-adaptable examples live in [examples/configs](../examples/configs):

| Example | Use when |
|---------|----------|
| `github-only.toml` | GitHub is the primary source for a personal review packet. |
| `github-gitlab-jira-manual.toml` | Work spans GitHub, GitLab, Jira, and hand-entered evidence. |
| `local-git-json-manual.toml` | You want a no-network local fixture/config pattern. |
| `public-portfolio.toml` | You are preparing a public-safe packet from local artifacts. |
