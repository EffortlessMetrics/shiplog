//! Compatibility wrapper for date-window utilities.
//!
//! This crate now delegates windowing logic to `shiplog-date-windows` while preserving
//! the historical public API surface of `shiplog_coverage`.

pub use shiplog_date_windows::{day_windows, month_windows, week_windows, window_len_days};
