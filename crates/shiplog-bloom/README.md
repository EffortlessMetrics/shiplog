# shiplog-bloom

Bloom filter implementation for shiplog.

## Usage

```rust
use shiplog_bloom::BloomFilter;

let mut filter: BloomFilter<String> = BloomFilter::new();
filter.insert(&"hello".to_string());
assert!(filter.contains(&"hello".to_string()));
```
