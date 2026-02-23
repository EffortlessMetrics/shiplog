# shiplog-hex

Hex encoding/decoding utilities for shiplog.

## Usage

```rust
use shiplog_hex::{encode, decode, encode_upper};

let encoded = encode(b"Hello");
assert_eq!(encoded, "48656c6c6f");

let decoded = decode(&encoded).unwrap();
assert_eq!(decoded, b"Hello");

let upper = encode_upper(b"Hello");
assert_eq!(upper, "48656C6C6F");
```
