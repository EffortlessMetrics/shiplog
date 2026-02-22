//! Trie data structure implementation for shiplog.
//!
//! This crate provides a Trie (prefix tree) implementation for efficient
//! string prefix operations and autocomplete scenarios.

use std::collections::HashMap;

/// A node in the Trie
#[derive(Debug, Clone)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end_of_word: bool,
    value: Option<String>,
}

impl TrieNode {
    fn new() -> Self {
        Self {
            children: HashMap::new(),
            is_end_of_word: false,
            value: None,
        }
    }
}

/// A Trie (prefix tree) data structure
#[derive(Debug, Clone)]
pub struct Trie {
    root: TrieNode,
    size: usize,
}

impl Trie {
    /// Creates a new empty Trie
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
            size: 0,
        }
    }

    /// Inserts a word into the Trie
    pub fn insert(&mut self, word: &str, value: Option<String>) {
        let mut current = &mut self.root;

        for c in word.chars() {
            current = current.children.entry(c).or_insert_with(TrieNode::new);
        }

        if !current.is_end_of_word {
            self.size += 1;
        }
        current.is_end_of_word = true;
        current.value = value;
    }

    /// Searches for a word in the Trie
    pub fn search(&self, word: &str) -> bool {
        self.find_node(word)
            .map(|node| node.is_end_of_word)
            .unwrap_or(false)
    }

    /// Returns the value associated with a word if it exists
    pub fn get(&self, word: &str) -> Option<&String> {
        self.find_node(word).and_then(|node| node.value.as_ref())
    }

    /// Checks if any word in the Trie starts with the given prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.find_node(prefix).is_some()
    }

    /// Returns all words that start with the given prefix
    pub fn autocomplete(&self, prefix: &str) -> Vec<String> {
        let mut results = Vec::new();

        if let Some(node) = self.find_node(prefix) {
            self.collect_words(prefix, node, &mut results);
        }

        results
    }

    /// Returns the number of words in the Trie
    pub fn len(&self) -> usize {
        self.size
    }

    /// Returns true if the Trie is empty
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Finds the node corresponding to a given prefix
    fn find_node(&self, prefix: &str) -> Option<&TrieNode> {
        let mut current = &self.root;

        for c in prefix.chars() {
            match current.children.get(&c) {
                Some(node) => current = node,
                None => return None,
            }
        }

        Some(current)
    }

    /// Recursively collects all words from a node
    fn collect_words(&self, prefix: &str, node: &TrieNode, results: &mut Vec<String>) {
        if node.is_end_of_word {
            results.push(prefix.to_string());
        }

        for (c, child) in &node.children {
            let new_prefix = format!("{}{}", prefix, c);
            self.collect_words(&new_prefix, child, results);
        }
    }
}

impl Default for Trie {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating Trie configurations
#[derive(Debug, Clone)]
pub struct TrieConfig {
    pub case_sensitive: bool,
}

impl Default for TrieConfig {
    fn default() -> Self {
        Self {
            case_sensitive: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trie_insert_search() {
        let mut trie = Trie::new();

        trie.insert("hello", Some("world".to_string()));
        trie.insert("hell", Some("lo".to_string()));
        trie.insert("help", Some("me".to_string()));

        assert!(trie.search("hello"));
        assert!(trie.search("hell"));
        assert!(trie.search("help"));
        assert!(!trie.search("hel"));
        assert!(!trie.search("hello world"));
    }

    #[test]
    fn test_trie_get() {
        let mut trie = Trie::new();

        trie.insert("hello", Some("world".to_string()));
        trie.insert("test", Some("value".to_string()));

        assert_eq!(trie.get("hello"), Some(&"world".to_string()));
        assert_eq!(trie.get("test"), Some(&"value".to_string()));
        assert_eq!(trie.get("notfound"), None);
    }

    #[test]
    fn test_trie_starts_with() {
        let mut trie = Trie::new();

        trie.insert("hello", None);
        trie.insert("hell", None);
        trie.insert("world", None);

        assert!(trie.starts_with("hel"));
        assert!(trie.starts_with("hell"));
        assert!(trie.starts_with("hello"));
        assert!(trie.starts_with("wor"));
        assert!(trie.starts_with("world"));
        assert!(!trie.starts_with("xyz"));
    }

    #[test]
    fn test_trie_autocomplete() {
        let mut trie = Trie::new();

        trie.insert("hello", None);
        trie.insert("hell", None);
        trie.insert("help", None);
        trie.insert("world", None);

        let hel_completions = trie.autocomplete("hel");
        assert!(hel_completions.contains(&"hello".to_string()));
        assert!(hel_completions.contains(&"hell".to_string()));
        assert!(hel_completions.contains(&"help".to_string()));

        let wor_completions = trie.autocomplete("wor");
        assert!(wor_completions.contains(&"world".to_string()));

        let xyz_completions = trie.autocomplete("xyz");
        assert!(xyz_completions.is_empty());
    }

    #[test]
    fn test_trie_len() {
        let mut trie = Trie::new();

        assert_eq!(trie.len(), 0);
        assert!(trie.is_empty());

        trie.insert("hello", None);
        assert_eq!(trie.len(), 1);

        trie.insert("world", None);
        assert_eq!(trie.len(), 2);

        // Inserting existing word doesn't increase size
        trie.insert("hello", None);
        assert_eq!(trie.len(), 2);
    }

    #[test]
    fn test_trie_empty_prefix() {
        let mut trie = Trie::new();

        trie.insert("a", None);
        trie.insert("b", None);

        let all = trie.autocomplete("");
        assert_eq!(all.len(), 2);
    }
}
