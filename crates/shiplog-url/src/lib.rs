//! URL parsing and utilities for shiplog.
//!
//! This crate provides URL parsing and utilities for the shiplog ecosystem.

use std::borrow::Cow;
use url::Url;

/// Parses a URL string and returns a Result
pub fn parse_url(url_str: &str) -> Result<Url, url::ParseError> {
    Url::parse(url_str)
}

/// Checks if a URL string is valid
pub fn is_valid_url(url_str: &str) -> bool {
    Url::parse(url_str).is_ok()
}

/// Gets the host from a URL
pub fn get_host(url: &Url) -> Option<&str> {
    url.host_str()
}

/// Gets the port from a URL
pub fn get_port(url: &Url) -> Option<u16> {
    url.port()
}

/// Gets the path from a URL
pub fn get_path(url: &Url) -> &str {
    url.path()
}

/// Gets the scheme (protocol) from a URL
pub fn get_scheme(url: &Url) -> &str {
    url.scheme()
}

/// Checks if URL uses HTTPS
pub fn is_https(url: &Url) -> bool {
    url.scheme() == "https"
}

/// Checks if URL uses HTTP
pub fn is_http(url: &Url) -> bool {
    url.scheme() == "http"
}

/// Gets a query parameter value by name
pub fn get_query_param<'a>(url: &'a Url, name: &str) -> Option<Cow<'a, str>> {
    url.query_pairs().find(|(key, _)| key == name).map(|(_, value)| value)
}

/// Builds a URL from parts
pub fn build_url(scheme: &str, host: &str, port: Option<u16>, path: &str) -> String {
    let mut url = format!("{}://{}", scheme, host);
    if let Some(p) = port {
        url.push_str(&format!(":{}", p));
    }
    if !path.is_empty() && !path.starts_with('/') {
        url.push('/');
    }
    url.push_str(path);
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_url() {
        let url = parse_url("https://example.com/path").unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn test_is_valid_url() {
        assert!(is_valid_url("https://example.com"));
        assert!(!is_valid_url("not-a-url"));
    }

    #[test]
    fn test_get_host() {
        let url = parse_url("https://example.com/path").unwrap();
        assert_eq!(get_host(&url), Some("example.com"));
    }

    #[test]
    fn test_get_port() {
        let url = parse_url("https://example.com:8080/path").unwrap();
        assert_eq!(get_port(&url), Some(8080));
    }

    #[test]
    fn test_get_path() {
        let url = parse_url("https://example.com/foo/bar").unwrap();
        assert_eq!(get_path(&url), "/foo/bar");
    }

    #[test]
    fn test_get_scheme() {
        let url = parse_url("https://example.com").unwrap();
        assert_eq!(get_scheme(&url), "https");
    }

    #[test]
    fn test_is_https() {
        let url = parse_url("https://example.com").unwrap();
        assert!(is_https(&url));
        
        let http_url = parse_url("http://example.com").unwrap();
        assert!(!is_https(&http_url));
    }

    #[test]
    fn test_is_http() {
        let url = parse_url("http://example.com").unwrap();
        assert!(is_http(&url));
    }

    #[test]
    fn test_get_query_param() {
        let url = parse_url("https://example.com?foo=bar&baz=qux").unwrap();
        assert_eq!(get_query_param(&url, "foo"), Some(std::borrow::Cow::Borrowed("bar")));
        assert_eq!(get_query_param(&url, "baz"), Some(std::borrow::Cow::Borrowed("qux")));
        assert_eq!(get_query_param(&url, "missing"), None);
    }

    #[test]
    fn test_build_url() {
        let url = build_url("https", "example.com", Some(8080), "/path");
        assert_eq!(url, "https://example.com:8080/path");
        
        let url_no_port = build_url("https", "example.com", None, "/path");
        assert_eq!(url_no_port, "https://example.com/path");
    }
}
