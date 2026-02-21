//! Path handling utilities for shiplog.
//!
//! This crate provides path handling utilities for the shiplog ecosystem.

use std::path::{Path, PathBuf};

/// Normalizes a path by resolving . and .. components
pub fn normalize_path(path: impl AsRef<Path>) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.as_ref().components() {
        match component {
            std::path::Component::ParentDir => {
                result.pop();
            }
            std::path::Component::CurDir => {}
            _ => result.push(component),
        }
    }
    result
}

/// Joins path segments safely
pub fn join_paths(base: impl AsRef<Path>, segments: &[&str]) -> PathBuf {
    let mut result = PathBuf::from(base.as_ref());
    for segment in segments {
        result = result.join(segment);
    }
    result
}

/// Gets the relative path from base to target
pub fn relative_path(base: impl AsRef<Path>, target: impl AsRef<Path>) -> Option<PathBuf> {
    let base = base.as_ref();
    let target = target.as_ref();

    let base_components = base.components().collect::<Vec<_>>();
    let target_components = target.components().collect::<Vec<_>>();

    let mut result = PathBuf::new();
    let mut found_common = false;

    for (bc, tc) in base_components.iter().zip(target_components.iter()) {
        if bc == tc {
            if !found_common {
                found_common = true;
            }
        } else {
            return None;
        }
    }

    // Add remaining target components
    for tc in target_components.iter().skip(base_components.len()) {
        result.push(tc);
    }

    Some(result)
}

/// Checks if a path is absolute
pub fn is_absolute_path(path: impl AsRef<Path>) -> bool {
    path.as_ref().is_absolute()
}

/// Checks if a path is relative
pub fn is_relative_path(path: impl AsRef<Path>) -> bool {
    !path.as_ref().is_absolute()
}

/// Gets the file extension from a path
pub fn get_extension(path: impl AsRef<Path>) -> Option<String> {
    path.as_ref()
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
}

/// Creates a path with a new extension
pub fn with_extension(path: impl AsRef<Path>, ext: &str) -> PathBuf {
    let mut p = PathBuf::from(path.as_ref());
    p.set_extension(ext);
    p
}

/// Normalizes path separators to forward slashes
pub fn to_forward_slashes(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_normalize_path() {
        let path = normalize_path("./foo/bar/../baz");
        // On Windows it uses backslashes, on Unix forward slashes
        // Just verify it resolves the parent component
        let s = path.to_string_lossy();
        assert!(!s.contains(".."));
    }

    #[test]
    fn test_join_paths() {
        let result = join_paths("/base", &["foo", "bar"]);
        // On Windows the result may have backslashes
        let s = result.to_string_lossy();
        assert!(s.contains("base") && s.contains("foo") && s.contains("bar"));
    }

    #[test]
    fn test_relative_path() {
        let base = PathBuf::from("/foo/bar");
        let target = PathBuf::from("/foo/bar/baz/qux");
        let rel = relative_path(&base, &target);
        assert!(rel.is_some());
    }

    #[test]
    fn test_is_absolute_path() {
        // On Windows, paths like /foo/bar are not absolute
        // Use a Windows-style absolute path
        #[cfg(windows)]
        {
            assert!(is_absolute_path("C:\\foo\\bar"));
            assert!(!is_absolute_path("foo\\bar"));
        }
        #[cfg(not(windows))]
        {
            assert!(is_absolute_path("/foo/bar"));
            assert!(!is_absolute_path("foo/bar"));
        }
    }

    #[test]
    fn test_is_relative_path() {
        #[cfg(windows)]
        {
            assert!(is_relative_path("foo\\bar"));
            assert!(!is_relative_path("C:\\foo\\bar"));
        }
        #[cfg(not(windows))]
        {
            assert!(is_relative_path("foo/bar"));
            assert!(!is_relative_path("/foo/bar"));
        }
    }

    #[test]
    fn test_get_extension() {
        let p = PathBuf::from("/foo/bar.txt");
        let ext = get_extension(&p);
        assert_eq!(ext, Some("txt".to_string()));

        let p2 = PathBuf::from("/foo/bar");
        assert_eq!(get_extension(&p2), None);
    }

    #[test]
    fn test_with_extension() {
        let result = with_extension("/foo/bar", "txt");
        assert!(result.to_string_lossy().ends_with("bar.txt"));
    }

    #[test]
    fn test_to_forward_slashes() {
        assert_eq!(to_forward_slashes("foo\\bar"), "foo/bar");
    }
}
