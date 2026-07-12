# shiplog.toml Reference

`shiplog.toml` is the repeat-use configuration for `shiplog collect multi` and
`shiplog intake`. It records default output, date-window, named review periods,
profile, and source settings. Secrets do not belong in this file; keep provider
tokens in environment variables.

Use these commands before collecting:

```bash
shiplog init --guided
shiplog config validate --config shiplog.toml
shiplog config explain --config shiplog.toml
shiplog doctor --config shiplog.toml
shiplog doctor --config shiplog.toml --setup
shiplog doctor --config shiplog.toml --setup --json
shiplog sources status --config shiplog.toml
shiplog sources list --config shiplog.toml
shiplog sources enable --source github --config shiplog.toml
shiplog sources disable --source jira --config shiplog.toml
shiplog status --latest
shiplog status --latest --json
shiplog doctor --config shiplog.toml --repair-plan
```

`init --guided` creates local-first setup files: local git when the current
directory is a repository, a valid manual journal, JSON import only when the
expected files exist, and token-backed provider sections left disabled until
their env vars and identities are configured. `config validate` checks the file
shape, configured date window, enabled source settings, enum values, and local
paths without requiring provider tokens.
`config explain` prints the resolved defaults and enabled sources. `doctor`
adds readiness checks such as token presence, identity discovery, output
writability, and redaction-key safety. `doctor --setup` is read-only and
no-network: it prints setup readiness, source state, credential presence,
share prerequisites, and read/write next actions without collecting data or
writing run artifacts. `doctor --setup --json` prints the same typed setup model
for agents and scripts. `sources status` is the source-only version of that
view: it prints canonical source keys, labels, enabled state, setup status,
reasons, and source next actions without share-profile noise. `sources list`
(with `--json`) is a lighter inventory that reports which `[sources.<name>]`
sections are present and whether each is enabled, without provider calls or
readiness checks. `sources enable --source <name>` and
`sources disable --source <name>` toggle the `enabled` flag for one or more
sources directly in `shiplog.toml`; they flip only that flag, preserve comments
and formatting, and never edit provider records (user, instance, owners) or
write tokens.
`doctor --repair-plan` is also read-only: it classifies setup issues and prints
source-by-source repair commands without collecting data or writing run
artifacts.
After setup is known, `status --latest` is the review-loop preflight. It reads
durable setup and run receipts, not `packet.md`, to summarize latest-run,
packet-readiness, repair, diff, share, and next-action state without writing.
Use `status --latest --json` when an agent or script needs the same cockpit
state without terminal-prose scraping. Add `status --check` for a cron/CI gate:
it prints the usual text (or `--json`) output and exits `0` when the loop is
ready or `1` when it needs action, reading receipts only.

Machine-readable control-plane contracts:

- [`Setup Readiness v1`](schemas/setup-readiness-v1.md) for
  `shiplog doctor --setup --json`.
- [`Review Loop Status v1`](schemas/review-loop-status-v1.md) for
  `shiplog status --latest --json`.
- [`GitHub Activity Harvest Receipts v1`](schemas/github-activity-harvest-v1.md)
  for `github.activity.plan.json`, `github.activity.progress.json`, and
  `github.activity.api-ledger.json`.
- [`GitHub Activity Report v1`](schemas/github-activity-report-v1.md) for
  `shiplog github activity merge`.

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
repo_owners = []
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
| `repo_owners` | Optional owner inclusion scope for actor-first GitHub harvests. Empty means actor-wide. Matching events are kept after actor search; shiplog does not crawl repositories. |
| `include_reviews` | Overrides `defaults.include_reviews`. |
| `no_details` | Skip detail fetches when true. |
| `throttle_ms` | Delay between provider requests. |
| `api_base` | API base URL; defaults to `https://api.github.com`. |
| `cache_dir` | Source cache directory. For GitHub this stores search probes, search pages, PR details, and PR review pages. |
| `no_cache` | Disable this source cache when true. |

### GitHub Activity Harvest

`shiplog github activity plan` reads the optional `[github_activity]` section
and writes `github.activity.plan.json` without calling GitHub, fetching PR
details, fetching review pages, rendering packets, or creating a cache. This is
the first receipt in the planned full-history harvest workflow. Use
`shiplog github activity scout` for the search-only scout profile, then
`shiplog github activity run --profile authored --resume` and
`shiplog github activity run --profile full --resume` as the cache warms.
After a run, `shiplog github activity status` reads the
plan/progress/API-ledger receipts without provider calls or writes.
`shiplog github activity report` writes `github.activity.report.json` and
`github.activity.report.md` from those receipts without provider calls.
`shiplog github activity merge` writes final activity outputs for the completed
run into `out/github-full/final/`.
For a full operator path, see the
[`GitHub activity harvest guide`](guides/github-activity-harvest.md).

```toml
[github_activity]
actor = "EffortlessSteven"
repo_owners = ["EffortlessMetrics", "EffortlessSteven"]
since = "2020-01-01"
until = "2026-05-20"
include_authored_prs = true
include_reviews = true
profile = "scout"
cache_dir = "./out/github-full/.cache"
cache_ttl_days = 3650

[github_activity.budget]
max_search_requests = 300
max_core_requests = 1000
max_search_per_minute = 24
on_exhausted = "checkpoint_and_stop"
```

| Field | Notes |
|-------|-------|
| `actor` | GitHub login to plan activity for. If omitted, `sources.github.user` is accepted as a compatibility alias. Static planning does not resolve `sources.github.me`. |
| `repo_owners` | Optional owner inclusion scope. The plan remains actor-first and records owner filtering as receipts; it does not crawl repositories. |
| `since` / `until` | Required harvest window. `until` is exclusive in shiplog's date model; generated search ranges record their exact inclusive GitHub query text. |
| `include_authored_prs` | Whether authored PR queries are planned. Defaults to `true`. |
| `include_reviews` | Used by the `full` profile to include review-candidate queries. |
| `profile` | `scout`, `authored`, or `full`; defaults to `scout`. Plan-only mode estimates the requested profile but does not spend API. |
| `cache_dir` | Used to infer the output root when `--out` is omitted and the value ends with `.cache`. Planning does not create the cache. |
| `cache_ttl_days` | Optional positive TTL override for the GitHub activity cache. Omit it to keep the normal source-cache TTL; set a large value for historical harvests whose old windows should stay reusable across resumes. |
| `budget.max_search_requests` | Planned search request ceiling. Defaults to `300`. |
| `budget.max_core_requests` | Planned core/detail request ceiling. Defaults to `1000`. |
| `budget.max_search_per_minute` | Planned search pacing ceiling. Defaults to `24`. |
| `budget.on_exhausted` | Currently must be `checkpoint_and_stop`. |

The generated plan is a static receipt. It estimates worst-case search/detail
cost from monthly actor-query windows and now includes the next executable
profile command. Scout/run commands write `github.activity.progress.json` and
`github.activity.api-ledger.json`; the ledger separates search/core request
counts, cache counts by phase, owner-filter receipts, and rate-limit event
arrays without token values. `--resume` skips valid completed windows from
`github.activity.windows/<profile>/<window_id>/`, carries cumulative API cost
forward, and checkpoints progress instead of silently continuing when budget is
exhausted. Activity merge also writes `github.activity.report.json` beside the
final packet, copied API ledger, and any intake report produced by the completed
activity run. The plan/progress/API-ledger receipts are pinned by
[`github.activity.*.v1`](schemas/github-activity-harvest-v1.md), and the merge
report is pinned by
[`github.activity.report.v1`](schemas/github-activity-report-v1.md).

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
does not inspect secrets. Use `shiplog doctor --setup` before collection or
rendering to catch missing redaction keys without writing share artifacts.

## Examples

Copy-adaptable examples live in [examples/configs](../examples/configs):

| Example | Use when |
|---------|----------|
| `github-only.toml` | GitHub is the primary source for a personal review packet. |
| `github-gitlab-jira-manual.toml` | Work spans GitHub, GitLab, Jira, and hand-entered evidence. |
| `local-git-json-manual.toml` | You want a no-network local fixture/config pattern. |
| `public-portfolio.toml` | You are preparing a public-safe packet from local artifacts. |
