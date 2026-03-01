# shiplog-batcher

Batching utilities for bulk shiplog operations.

## Usage

```rust
use shiplog_batcher::{Batcher, BatchConfig};

let config = BatchConfig { max_size: 100, flush_timeout_ms: 5000 };
let batcher = Batcher::new(config);
batcher.add(item);
let batch = batcher.flush();
```

## Features

- `BatchConfig` — configurable batch size and flush timeout
- `Batcher<T>` — generic batch collector
- `BatchProcessor<T>` — processes complete batches
- `BatchItem<T>` — wrapper for batched items with metadata

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
