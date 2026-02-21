//! Enumeration utilities for shiplog.
//!
//! This crate provides enumeration utilities for iterating with indices and labels.

/// Enumeration config
#[derive(Debug, Clone)]
pub struct EnumerationConfig {
    pub start_index: usize,
    pub step: usize,
}

impl Default for EnumerationConfig {
    fn default() -> Self {
        Self {
            start_index: 0,
            step: 1,
        }
    }
}

/// Builder for enumeration configurations
#[derive(Debug)]
pub struct EnumerationBuilder {
    config: EnumerationConfig,
}

impl EnumerationBuilder {
    pub fn new() -> Self {
        Self {
            config: EnumerationConfig::default(),
        }
    }

    pub fn start_index(mut self, index: usize) -> Self {
        self.config.start_index = index;
        self
    }

    pub fn step(mut self, step: usize) -> Self {
        self.config.step = step;
        self
    }

    pub fn build(self) -> EnumerationConfig {
        self.config
    }
}

impl Default for EnumerationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// An iterator that enumerates items with index
pub struct Enumerate<I> {
    iter: I,
    index: usize,
}

impl<I> Enumerate<I> {
    pub fn new(iter: I) -> Self {
        Self { iter, index: 0 }
    }

    pub fn with_start(iter: I, start: usize) -> Self {
        Self { iter, index: start }
    }
}

impl<I: Iterator> Iterator for Enumerate<I> {
    type Item = (usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) => {
                let idx = self.index;
                self.index += 1;
                Some((idx, item))
            }
            None => None,
        }
    }
}

/// Labeled enumeration with custom labels
pub struct LabeledEnumerate<I> {
    iter: I,
    index: usize,
    labels: Vec<String>,
}

impl<I: Iterator> LabeledEnumerate<I> {
    pub fn new(iter: I, labels: Vec<String>) -> Self {
        Self {
            iter,
            index: 0,
            labels,
        }
    }

    pub fn with_generator<G>(iter: I, label_generator: G) -> Self
    where
        G: Fn(usize) -> String,
    {
        // Pre-generate labels based on iterator size hint (worst case)
        let labels: Vec<String> = (0..1000).map(label_generator).collect();
        Self {
            iter,
            index: 0,
            labels,
        }
    }
}

impl<I: Iterator> Iterator for LabeledEnumerate<I> {
    type Item = (String, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        match self.iter.next() {
            Some(item) => {
                let label = self
                    .labels
                    .get(self.index)
                    .cloned()
                    .unwrap_or_else(|| format!("item_{}", self.index));
                self.index += 1;
                Some((label, item))
            }
            None => None,
        }
    }
}

/// Extension trait for enumeration
pub trait EnumerateExt: Iterator + Sized {
    fn enumerate_items(self) -> Enumerate<Self>;
    fn enumerate_from(self, start: usize) -> Enumerate<Self>;
    fn enumerate_labeled(self, labels: Vec<String>) -> LabeledEnumerate<Self>;
}

impl<I: Iterator + Sized> EnumerateExt for I {
    fn enumerate_items(self) -> Enumerate<Self> {
        Enumerate::new(self)
    }

    fn enumerate_from(self, start: usize) -> Enumerate<Self> {
        Enumerate::with_start(self, start)
    }

    fn enumerate_labeled(self, labels: Vec<String>) -> LabeledEnumerate<Self> {
        LabeledEnumerate::new(self, labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumeration_config_default() {
        let config = EnumerationConfig::default();
        assert_eq!(config.start_index, 0);
        assert_eq!(config.step, 1);
    }

    #[test]
    fn test_enumeration_builder() {
        let config = EnumerationBuilder::new().start_index(5).step(2).build();

        assert_eq!(config.start_index, 5);
        assert_eq!(config.step, 2);
    }

    #[test]
    fn test_enumerate() {
        let items = vec!["a", "b", "c"];
        let enumerated: Vec<_> = items.into_iter().enumerate_items().collect();

        assert_eq!(enumerated, vec![(0, "a"), (1, "b"), (2, "c")]);
    }

    #[test]
    fn test_enumerate_from() {
        let items = vec!["a", "b", "c"];
        let enumerated: Vec<_> = items.into_iter().enumerate_from(10).collect();

        assert_eq!(enumerated, vec![(10, "a"), (11, "b"), (12, "c")]);
    }

    #[test]
    fn test_labeled_enumerate() {
        let items = vec!["a", "b", "c"];
        let labels = vec![
            "first".to_string(),
            "second".to_string(),
            "third".to_string(),
        ];
        let enumerated: Vec<_> = items.into_iter().enumerate_labeled(labels).collect();

        assert_eq!(
            enumerated,
            vec![
                ("first".to_string(), "a"),
                ("second".to_string(), "b"),
                ("third".to_string(), "c"),
            ]
        );
    }
}
