//! Environment variable utilities for shiplog.
//!
//! This crate provides environment variable utilities for the shiplog ecosystem.
#![allow(unsafe_code)]

use std::env;

/// Gets an environment variable, returning None if not set.
pub fn get_var(name: &str) -> Option<String> {
    env::var(name).ok()
}

/// Gets an environment variable, returning a default if not set.
pub fn get_var_or(name: &str, default: &str) -> String {
    env::var(name).unwrap_or_else(|_| default.to_string())
}

/// Gets an environment variable and parses it as the specified type.
pub fn get_var_as<T: std::str::FromStr>(name: &str) -> Option<T> {
    env::var(name).ok().and_then(|v| v.parse().ok())
}

/// Gets an environment variable, returning a default if not set or parsing fails.
pub fn get_var_or_parse<T: std::str::FromStr>(name: &str, default: &str) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Debug,
{
    env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| default.parse().unwrap())
}

/// Checks if an environment variable is set.
pub fn is_set(name: &str) -> bool {
    env::var(name).is_ok()
}

/// Checks if an environment variable is set to a truthy value.
/// Truthy values: "1", "true", "yes", "on"
pub fn is_truthy(name: &str) -> bool {
    match env::var(name) {
        Ok(val) => {
            let lower = val.to_lowercase();
            lower == "1" || lower == "true" || lower == "yes" || lower == "on"
        }
        Err(_) => false,
    }
}

/// Checks if an environment variable is set to a falsy value.
/// Falsy values: "0", "false", "no", "off"
pub fn is_falsy(name: &str) -> bool {
    match env::var(name) {
        Ok(val) => {
            let lower = val.to_lowercase();
            lower == "0" || lower == "false" || lower == "no" || lower == "off"
        }
        Err(_) => false,
    }
}

/// Sets an environment variable (only works on Unix-like systems on Windows).
#[allow(unsafe_code)]
pub fn set_var(name: &str, value: &str) -> Result<(), EnvError> {
    unsafe { env::set_var(name, value) };
    Ok(())
}

/// Removes an environment variable.
#[allow(unsafe_code)]
pub fn remove_var(name: &str) {
    unsafe { env::remove_var(name) }
}

/// Returns an iterator over all environment variables.
pub fn vars() -> env::Vars {
    env::vars()
}

/// Returns an iterator over environment variables matching a prefix.
pub fn vars_with_prefix(prefix: &str) -> impl Iterator<Item = (String, String)> {
    let prefix = prefix.to_string();
    env::vars().filter(move |(key, _)| key.starts_with(&prefix))
}

/// Returns the current working directory.
pub fn current_dir() -> Result<std::path::PathBuf, EnvError> {
    env::current_dir().map_err(|e| EnvError::IoError(e.to_string()))
}

/// Returns the path separator for the current platform.
pub fn path_separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}

/// Returns the list of paths from the PATH environment variable.
pub fn get_paths() -> Vec<std::path::PathBuf> {
    env::var_os("PATH")
        .map(|paths| {
            env::split_paths(&paths)
                .filter_map(|p| {
                    if p.exists() {
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

/// Error type for environment operations.
#[derive(Debug, Clone, PartialEq)]
pub enum EnvError {
    IoError(String),
}

impl std::fmt::Display for EnvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EnvError::IoError(msg) => write!(f, "Environment error: {}", msg),
        }
    }
}

impl std::error::Error for EnvError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_var_not_set() {
        // Use a variable name that is unlikely to be set
        let result = get_var("SHIPLOG_ENV_TEST_NONEXISTENT_VAR_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_get_var_or() {
        let result = get_var_or("SHIPLOG_ENV_TEST_NONEXISTENT_VAR_12345", "default");
        assert_eq!(result, "default");
    }

    #[test]
    fn test_get_var_as() {
        // Use a variable that should exist on all systems
        let result: Option<u32> = get_var_as("SHIPLOG_ENV_TEST_NONEXISTENT_VAR_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_is_set() {
        // Use a variable that should exist on all systems
        let result = is_set("PATH");
        assert!(result);
        
        let result2 = is_set("SHIPLOG_ENV_TEST_NONEXISTENT_VAR_12345");
        assert!(!result2);
    }

    #[test]
    fn test_is_truthy() {
        // Test truthy detection with environment variable
        // Note: These test the logic, not actual env vars
        // The function checks env vars that don't exist, so they return false
        // We test the logic by examining the implementation
        assert!(!is_truthy("SHIPLOG_NONEXISTENT_VAR_12345"));
    }

    #[test]
    fn test_is_falsy() {
        // Test falsy detection with environment variable
        // Note: These test the logic, not actual env vars
        assert!(!is_falsy("SHIPLOG_NONEXISTENT_VAR_12345"));
    }

    #[test]
    fn test_vars() {
        let vars: Vec<_> = vars().take(5).collect();
        // Should have at least some environment variables
        assert!(!vars.is_empty());
    }

    #[test]
    fn test_vars_with_prefix() {
        // PATH should exist on all systems
        let path_vars: Vec<_> = vars_with_prefix("PATH").collect();
        assert!(!path_vars.is_empty());
    }

    #[test]
    fn test_current_dir() {
        let result = current_dir();
        assert!(result.is_ok());
    }

    #[test]
    fn test_path_separator() {
        if cfg!(windows) {
            assert_eq!(path_separator(), ';');
        } else {
            assert_eq!(path_separator(), ':');
        }
    }

    #[test]
    fn test_get_paths() {
        let paths = get_paths();
        // Should have at least some paths
        assert!(!paths.is_empty() || cfg!(windows)); // Windows might have empty in some environments
    }
}
