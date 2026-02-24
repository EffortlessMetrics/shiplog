# shiplog-redaction-repo

Single-responsibility repository redaction contract for shiplog.

This crate owns one contract:

- redact `RepoRef` values for the `public` profile by aliasing `full_name`,
  removing `html_url`, and forcing visibility to `Unknown`

It intentionally contains no profile parsing, event/workstream transforms, or
alias-cache persistence.
