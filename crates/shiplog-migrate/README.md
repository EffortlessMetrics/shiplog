# shiplog-migrate

Schema migration utilities for shiplog.

## Usage

```rust
use shiplog_migrate::{MigrationRunner, Migration};

let runner = MigrationRunner::new();
runner.add(Migration::new("001_initial", |state| { /* ... */ Ok(()) }));
runner.run(&mut schema_state)?;
```

## Features

- `Migration` — a named migration step with an apply function
- `MigrationRunner` — executes migrations in sequence
- `MigrationRecord` — tracks which migrations have been applied
- `SchemaState` — current schema version and status

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
