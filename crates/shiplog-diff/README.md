# shiplog-diff

Diff algorithms for comparing shiplog events and packets.

## Usage

```rust
use shiplog_diff::{diff_events, diff_summary};

let changes = diff_events(&old_events, &new_events);
let summary = diff_summary(&changes);
println!("Added: {}, Removed: {}, Modified: {}", summary.added, summary.removed, summary.modified);
```

## Features

- `EventDiff` — describes a single event change
- `EventChange` — added, removed, or modified event
- `FieldChange` — field-level before/after tracking
- `DiffSummary` — aggregate change statistics
- `diff_events()` / `diff_summary()` — compare event sets

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
