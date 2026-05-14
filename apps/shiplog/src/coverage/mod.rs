#![warn(missing_docs)]
//! Date-window utilities for coverage and ingestion slicing.
//!
//! This module owns the coverage-facing windowing primitives that keep query
//! windows explicit, contiguous, and auditable.
//!
//! # Examples
//!
//! ```
//! use shiplog::coverage::{month_windows, week_windows, day_windows, window_len_days};
//! use chrono::NaiveDate;
//!
//! let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
//! let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
//!
//! let months = month_windows(since, until);
//! assert_eq!(months.len(), 3); // Jan, Feb, Mar
//!
//! assert_eq!(window_len_days(&months[0]), 31);
//! ```

mod windows;

pub use windows::{day_windows, month_windows, week_windows, window_len_days};
