# shiplog-filter

Filtering logic for shiplog events based on criteria.

## Usage

```rust
use shiplog_filter::{EventFilter, filter_events};

let filter = EventFilter::new()
    .source("github")
    .kind("pr_merged");
let filtered = filter_events(&events, &filter);
```

## Features

- `EventFilter` — multi-criteria filter (source, kind, date range, actor, tags, repo)
- `filter_events()` — apply a filter to an event collection

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
