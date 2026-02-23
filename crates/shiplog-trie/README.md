# shiplog-trie

Trie (prefix tree) data structure implementation for shiplog.

## Usage

```rust
use shiplog_trie::Trie;

let mut trie = Trie::new();
trie.insert("hello", Some("world".to_string()));
assert!(trie.search("hello"));
assert_eq!(trie.get("hello"), Some(&"world".to_string()));
```
