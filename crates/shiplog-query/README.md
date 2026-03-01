# shiplog-query

Query language and filter syntax for searching shiplog events.

## Usage

```rust
use shiplog_query::{Query, query_events};

let query = Query::parse("source:github kind:pr_merged since:2025-01-01")?;
let results = query_events(&events, &query);
```

## Features

- `Query` — parsed query with filter predicates
- `QueryError` — structured parse error type
- `query_events()` — apply a query to an event collection
- Supported filters: `source:`, `kind:`, `actor:`, `tag:`, `repo:`, `since:`, `until:`

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
