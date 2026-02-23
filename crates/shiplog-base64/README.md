# shiplog-base64

Base64 encoding/decoding utilities for shiplog.

## Usage

```rust
use shiplog_base64::{encode, decode, encode_string, decode_to_string};

let encoded = encode(b"Hello, World!");
assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");

let decoded = decode(&encoded).unwrap();
assert_eq!(decoded, b"Hello, World!");

// String variants
let encoded = encode_string("Hello");
assert_eq!(encoded, "SGVsbG8");

let decoded = decode_to_string(&encoded).unwrap();
assert_eq!(decoded, "Hello");
```
