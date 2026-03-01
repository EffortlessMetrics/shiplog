# shiplog-cache-sqlite

SQLite-backed cache persistence microcrate for shiplog.

This crate is focused on one responsibility:

- managing durable SQLite tables for cached API responses,
- storing and retrieving typed JSON values with TTL enforcement,
- computing cache visibility contracts (`contains`, `stats`, cleanup).

`shiplog-cache` depends on this crate and re-exports `ApiCache` as the public façade.
