# shiplog-compressor

Compression utilities for shiplog data.

## Usage

```rust
use shiplog_compressor::{Compressor, CompressionAlgorithm};

let compressor = Compressor::new(CompressionAlgorithm::Gzip);
let compressed = compressor.compress(data)?;
let original = compressor.decompress(&compressed)?;
```

## Features

- `CompressionAlgorithm` — supported algorithms (Gzip, Snappy)
- `Compressor` — compress and decompress with a chosen algorithm
- `CompressionStats` — tracks compression ratios and sizes

## Part of the shiplog workspace

See the [workspace README](../../README.md) for overall architecture.

## License

Licensed under either of [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
or [MIT license](http://opensource.org/licenses/MIT) at your option.
