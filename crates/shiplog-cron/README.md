# shiplog-cron

Continuous/cron mode for scheduled shiplog collection.

## Usage

```rust
use shiplog_cron::{CronConfig, CronScheduler};

let config = CronConfig { expression: "0 0 * * *".into(), ..Default::default() };
let scheduler = CronScheduler::new(config);
scheduler.start()?;
```

## Features

- `CronConfig` — schedule configuration with cron expressions
- `CronScheduler` — runs shiplog collection on a schedule

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
