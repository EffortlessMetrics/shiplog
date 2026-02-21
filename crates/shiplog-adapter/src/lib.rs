//! Adapter pattern utilities for shiplog.
//!
//! This crate provides utilities for implementing the adapter pattern to bridge different interfaces.

use std::fmt;

/// Target trait that our code expects
pub trait Target {
    fn request(&self) -> String;
}

/// Adaptee - the interface we want to adapt
pub trait Adaptee {
    fn specific_request(&self) -> String;
}

/// Concrete adaptee implementation
#[derive(Debug)]
pub struct ConcreteAdaptee {
    value: String,
}

impl ConcreteAdaptee {
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl Adaptee for ConcreteAdaptee {
    fn specific_request(&self) -> String {
        format!("Adaptee: {}", self.value)
    }
}

/// Adapter that implements Target while wrapping an Adaptee
pub struct Adapter<T: Adaptee> {
    adaptee: T,
}

impl<T: Adaptee> Adapter<T> {
    pub fn new(adaptee: T) -> Self {
        Self { adaptee }
    }
}

impl<T: Adaptee> Target for Adapter<T> {
    fn request(&self) -> String {
        format!("Adapter: ({}))", self.adaptee.specific_request())
    }
}

/// Builder for creating adapters
#[derive(Debug)]
pub struct AdapterBuilder {
    name: String,
}

impl AdapterBuilder {
    pub fn new() -> Self {
        Self {
            name: "adapter".to_string(),
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn build<T: Adaptee>(self, adaptee: T) -> NamedAdapter<T> {
        NamedAdapter {
            name: self.name,
            adaptee,
        }
    }
}

impl Default for AdapterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Named adapter with additional metadata
pub struct NamedAdapter<T: Adaptee> {
    name: String,
    adaptee: T,
}

impl<T: Adaptee> NamedAdapter<T> {
    pub fn new(name: &str, adaptee: T) -> Self {
        Self {
            name: name.to_string(),
            adaptee,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn adapt(&self) -> String {
        format!("[{}] {}", self.name, self.adaptee.specific_request())
    }
}

impl<T: Adaptee + fmt::Debug> fmt::Debug for NamedAdapter<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NamedAdapter")
            .field("name", &self.name)
            .field("adaptee", &self.adaptee)
            .finish()
    }
}

/// Wrapper for adapting between different types
pub struct TypeAdapter<S, T> {
    source: S,
    converter: fn(&S) -> T,
}

impl<S, T> TypeAdapter<S, T> {
    pub fn new(source: S, converter: fn(&S) -> T) -> Self {
        Self { source, converter }
    }

    pub fn adapt(&self) -> T {
        (self.converter)(&self.source)
    }
}

/// Converter trait for generic type adaptation
pub trait Converter<Input, Output> {
    fn convert(&self, input: &Input) -> Output;
}

/// Generic converter implementation
pub struct GenericConverter<F> {
    func: F,
}

impl<F, Input, Output> Converter<Input, Output> for GenericConverter<F>
where
    F: Fn(&Input) -> Output,
{
    fn convert(&self, input: &Input) -> Output {
        (self.func)(input)
    }
}

impl<F> GenericConverter<F> {
    pub fn new(func: F) -> Self {
        Self { func }
    }
}

/// Two-way adapter for bidirectional conversion
pub trait TwoWayAdapter<A, B> {
    fn adapt_to(&self, a: &A) -> B;
    fn adapt_from(&self, b: &B) -> A;
}

/// Simple two-way converter
pub struct BidirectionalConverter;

impl TwoWayAdapter<String, i64> for BidirectionalConverter {
    fn adapt_to(&self, a: &String) -> i64 {
        a.parse().unwrap_or(0)
    }

    fn adapt_from(&self, b: &i64) -> String {
        b.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concrete_adaptee() {
        let adaptee = ConcreteAdaptee::new("test");
        assert_eq!(adaptee.specific_request(), "Adaptee: test");
    }

    #[test]
    fn test_adapter() {
        let adaptee = ConcreteAdaptee::new("test");
        let adapter = Adapter::new(adaptee);
        assert_eq!(adapter.request(), "Adapter: (Adaptee: test))");
    }

    #[test]
    fn test_adapter_builder() {
        let adapter = AdapterBuilder::new()
            .name("my-adapter")
            .build(ConcreteAdaptee::new("value"));
        
        assert_eq!(adapter.name(), "my-adapter");
        assert_eq!(adapter.adapt(), "[my-adapter] Adaptee: value");
    }

    #[test]
    fn test_named_adapter_debug() {
        let adaptee = ConcreteAdaptee::new("debug-test");
        let adapter = NamedAdapter::new("test", adaptee);
        
        let debug_str = format!("{:?}", adapter);
        assert!(debug_str.contains("NamedAdapter"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_type_adapter() {
        let adapter = TypeAdapter::new(42, |i: &i32| i.to_string());
        assert_eq!(adapter.adapt(), "42");
    }

    #[test]
    fn test_generic_converter() {
        let converter = GenericConverter::new(|s: &String| s.len());
        assert_eq!(converter.convert(&"hello".to_string()), 5);
    }

    #[test]
    fn test_bidirectional_converter_to() {
        let converter = BidirectionalConverter;
        let result: i64 = converter.adapt_to(&"123".to_string());
        assert_eq!(result, 123);
    }

    #[test]
    fn test_bidirectional_converter_from() {
        let converter = BidirectionalConverter;
        let result: String = converter.adapt_from(&456);
        assert_eq!(result, "456");
    }

    #[test]
    fn test_adapter_builder_default() {
        let adapter = AdapterBuilder::default()
            .build(ConcreteAdaptee::new("default"));
        
        assert_eq!(adapter.name(), "adapter");
    }
}
