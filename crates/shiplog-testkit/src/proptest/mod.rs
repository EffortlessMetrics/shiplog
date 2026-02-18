//! Proptest strategies for shiplog property-based testing
//!
//! This module provides reusable proptest strategies for generating valid test data
//! across all shiplog crates.

pub mod strategies;
pub mod roadmap_strategies;

#[cfg(test)]
mod roadmap_property_tests;

pub use strategies::*;
pub use roadmap_strategies::*;
