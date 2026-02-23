# shiplog-crypto

Cryptographic utilities for shiplog.

## Overview

This crate provides cryptographic utilities for the shiplog project, including:
- SHA-256 and SHA-512 hashing
- Hash verification
- Basic XOR cipher for obfuscation

## Usage

```rust
use shiplog_crypto::{Hash, hash_string, verify_hash};

// Compute a hash
let hash = Hash::sha256(b"hello world");
println!("Hash: {}", hash.value);

// Verify data against a hash
assert!(hash.verify(b"hello world"));

// Convenience functions
let hash = hash_string("my string");
assert!(verify_hash("my string", &hash));
```

## License

MIT OR Apache-2.0
