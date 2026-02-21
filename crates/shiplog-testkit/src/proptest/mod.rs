//! Proptest strategies for shiplog property-based testing
//!
//! This module provides reusable proptest strategies for generating valid test data
//! across all shiplog crates.

pub mod strategies;

pub use strategies::*;
