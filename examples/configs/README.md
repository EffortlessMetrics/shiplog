# Example shiplog configs

These examples are starting points for `shiplog.toml`. Copy one to your repo
root, edit the placeholder identities and instances, then run:

```bash
shiplog config validate --config shiplog.toml
shiplog config explain --config shiplog.toml
shiplog doctor --config shiplog.toml
```

Tokens stay in environment variables such as `GITHUB_TOKEN`, `GITLAB_TOKEN`,
`JIRA_TOKEN`, `LINEAR_API_KEY`, and `SHIPLOG_REDACT_KEY`; they do not belong in
`shiplog.toml`.

For every supported section and field, see
[docs/config-reference.md](../../docs/config-reference.md).

Configs with `profile = "manager"` or `profile = "public"` validate without a
redaction key, but `doctor`, collection, rendering, and bundle creation require
`SHIPLOG_REDACT_KEY` or the configured redaction key environment variable.

The fixture-safe local example can be checked from this repository without live
API credentials:

```bash
shiplog config validate --config examples/configs/local-git-json-manual.toml
shiplog config explain --config examples/configs/local-git-json-manual.toml
```

| File | Use when |
|------|----------|
| `github-only.toml` | GitHub is the primary source for a personal review packet. |
| `github-gitlab-jira-manual.toml` | Work spans GitHub, GitLab, Jira, and hand-entered evidence. |
| `local-git-json-manual.toml` | You want a no-network local fixture/config pattern. |
| `public-portfolio.toml` | You are preparing a public-safe packet from local artifacts. |
