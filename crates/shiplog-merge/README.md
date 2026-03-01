# shiplog-merge

Merging utilities for combining multiple event sources.

## Usage

```rust
use shiplog_merge::{merge_events, MergeStrategy, ConflictResolution};

let strategy = MergeStrategy { conflict: ConflictResolution::PreferRecent };
let result = merge_events(&source_a, &source_b, &strategy)?;
println!("Merged: {}, Conflicts: {}", result.merged, result.conflicts);
```

## Features

- `ConflictResolution` — strategies: PreferFirst, PreferRecent, PreferComplete
- `MergeStrategy` — configurable merge behavior
- `MergeReport` / `MergeResult` — merge outcome with conflict details
- `merge_events()` — merge two event sets with deduplication
- `merge_ingest_outputs()` — merge complete ingest outputs

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
