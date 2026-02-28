//! Compatibility wrapper for date-window utilities.
//!
//! This crate now delegates windowing logic to `shiplog-date-windows` while preserving
//! the historical public API surface of `shiplog_coverage`.
//!
//! # Examples
//!
//! ```
//! use shiplog_coverage::{month_windows, week_windows, day_windows, window_len_days};
//! use chrono::NaiveDate;
//!
//! let since = NaiveDate::from_ymd_opt(2025, 1, 1).unwrap();
//! let until = NaiveDate::from_ymd_opt(2025, 4, 1).unwrap();
//!
//! let months = month_windows(since, until);
//! assert_eq!(months.len(), 3); // Jan, Feb, Mar
//!
//! // Each window knows its length:
//! assert_eq!(window_len_days(&months[0]), 31); // January
//! ```

pub use shiplog_date_windows::{day_windows, month_windows, week_windows, window_len_days};
