# shiplog-watch

File system watching for changes in shiplog.

## Usage

```rust
use shiplog_watch::{FileWatcher, WatchConfig, watch_directory};

let config = WatchConfig::default();
let watcher = watch_directory("./out", config)?;
for change in watcher.changes() {
    println!("{:?}: {}", change.event_type, change.path.display());
}
```

## Features

- `FileEventType` — create, modify, remove, rename events
- `FileChange` — describes a single file system change
- `WatchConfig` — debounce and filter configuration
- `FileWatcher` / `watch_directory()` — monitor a directory for changes

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
