//! Integration tests for shiplog-url.

use proptest::prelude::*;
use shiplog_url::*;

// ── RFC compliance / known-answer tests ─────────────────────────

#[test]
fn parse_url_https() {
    let url = parse_url("https://example.com/path?q=1#frag").unwrap();
    assert_eq!(get_scheme(&url), "https");
    assert_eq!(get_host(&url), Some("example.com"));
    assert_eq!(get_path(&url), "/path");
}

#[test]
fn parse_url_with_port() {
    let url = parse_url("http://localhost:8080/api").unwrap();
    assert_eq!(get_host(&url), Some("localhost"));
    assert_eq!(get_port(&url), Some(8080));
    assert_eq!(get_path(&url), "/api");
}

#[test]
fn parse_url_with_userinfo() {
    let url = parse_url("https://user:pass@example.com/path").unwrap();
    assert_eq!(get_host(&url), Some("example.com"));
}

#[test]
fn parse_url_ftp() {
    let url = parse_url("ftp://files.example.com/data").unwrap();
    assert_eq!(get_scheme(&url), "ftp");
}

#[test]
fn parse_url_invalid() {
    assert!(parse_url("not-a-url").is_err());
    assert!(parse_url("").is_err());
    assert!(parse_url("://missing-scheme").is_err());
}

#[test]
fn is_valid_url_tests() {
    assert!(is_valid_url("https://example.com"));
    assert!(is_valid_url("http://localhost:3000"));
    assert!(is_valid_url("ftp://files.example.com"));
    assert!(!is_valid_url("not a url"));
    assert!(!is_valid_url(""));
    assert!(!is_valid_url("example.com")); // no scheme
}

// ── Scheme tests ────────────────────────────────────────────────

#[test]
fn is_https_tests() {
    let https = parse_url("https://example.com").unwrap();
    assert!(is_https(&https));
    assert!(!is_http(&https));

    let http = parse_url("http://example.com").unwrap();
    assert!(is_http(&http));
    assert!(!is_https(&http));
}

// ── Query parameter tests ───────────────────────────────────────

#[test]
fn get_query_param_basic() {
    let url = parse_url("https://example.com?foo=bar&baz=qux").unwrap();
    assert_eq!(
        get_query_param(&url, "foo").map(|c| c.into_owned()),
        Some("bar".to_string())
    );
    assert_eq!(
        get_query_param(&url, "baz").map(|c| c.into_owned()),
        Some("qux".to_string())
    );
    assert!(get_query_param(&url, "missing").is_none());
}

#[test]
fn get_query_param_encoded() {
    let url = parse_url("https://example.com?key=hello%20world").unwrap();
    let val = get_query_param(&url, "key").map(|c| c.into_owned());
    assert_eq!(val, Some("hello world".to_string()));
}

#[test]
fn get_query_param_empty_value() {
    let url = parse_url("https://example.com?key=").unwrap();
    let val = get_query_param(&url, "key").map(|c| c.into_owned());
    assert_eq!(val, Some("".to_string()));
}

#[test]
fn get_query_param_no_query() {
    let url = parse_url("https://example.com/path").unwrap();
    assert!(get_query_param(&url, "key").is_none());
}

// ── build_url tests ─────────────────────────────────────────────

#[test]
fn build_url_with_port() {
    assert_eq!(
        build_url("https", "example.com", Some(8080), "/api"),
        "https://example.com:8080/api"
    );
}

#[test]
fn build_url_without_port() {
    assert_eq!(
        build_url("https", "example.com", None, "/path"),
        "https://example.com/path"
    );
}

#[test]
fn build_url_empty_path() {
    assert_eq!(
        build_url("https", "example.com", None, ""),
        "https://example.com"
    );
}

#[test]
fn build_url_path_without_leading_slash() {
    assert_eq!(
        build_url("https", "example.com", None, "api/v1"),
        "https://example.com/api/v1"
    );
}

#[test]
fn build_url_path_with_leading_slash() {
    assert_eq!(
        build_url("https", "example.com", None, "/api/v1"),
        "https://example.com/api/v1"
    );
}

// ── Edge cases ──────────────────────────────────────────────────

#[test]
fn parse_url_ipv4() {
    let url = parse_url("http://192.168.1.1:3000/api").unwrap();
    assert_eq!(get_host(&url), Some("192.168.1.1"));
    assert_eq!(get_port(&url), Some(3000));
}

#[test]
fn parse_url_ipv6() {
    let url = parse_url("http://[::1]:8080/api").unwrap();
    assert_eq!(get_host(&url), Some("[::1]"));
    assert_eq!(get_port(&url), Some(8080));
}

#[test]
fn parse_url_long_path() {
    let long_path = "/a".repeat(1000);
    let url_str = format!("https://example.com{}", long_path);
    let url = parse_url(&url_str).unwrap();
    assert_eq!(get_path(&url), long_path);
}

#[test]
fn parse_url_unicode_host() {
    // IDN (international domain name) - url crate handles these
    let result = parse_url("https://例え.jp/path");
    assert!(result.is_ok());
}

// ── Snapshot tests ──────────────────────────────────────────────

#[test]
fn snapshot_build_url_matrix() {
    let cases = [
        ("https", "example.com", Some(443), "/"),
        ("http", "localhost", Some(8080), "/api/v1"),
        ("https", "example.com", None, "/path"),
        ("ftp", "files.example.com", None, "/data"),
        ("https", "example.com", None, ""),
    ];
    let formatted: Vec<String> = cases
        .iter()
        .map(|(scheme, host, port, path)| {
            format!(
                "{}://{}:{:?}{} => {}",
                scheme,
                host,
                port,
                path,
                build_url(scheme, host, *port, path)
            )
        })
        .collect();
    insta::assert_snapshot!(formatted.join("\n"));
}

// ── Property tests ──────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_build_url_starts_with_scheme(
        scheme in "(https|http|ftp)",
        host in "[a-z]{3,10}\\.[a-z]{2,4}",
    ) {
        let url = build_url(&scheme, &host, None, "/");
        let expected_prefix = format!("{}://", scheme);
        prop_assert!(url.starts_with(&expected_prefix));
    }

    #[test]
    fn prop_build_url_contains_host(
        host in "[a-z]{3,10}\\.[a-z]{2,4}",
    ) {
        let url = build_url("https", &host, None, "/");
        prop_assert!(url.contains(&host));
    }

    #[test]
    fn prop_build_url_with_port_contains_port(
        port in 1u16..65535,
    ) {
        let url = build_url("https", "example.com", Some(port), "/");
        let port_str = format!(":{}", port);
        prop_assert!(url.contains(&port_str));
    }

    #[test]
    fn prop_valid_url_parses(
        host in "[a-z]{3,8}\\.[a-z]{2,3}",
        path in "/[a-z]{0,10}",
    ) {
        let url_str = format!("https://{}{}", host, path);
        let result = parse_url(&url_str);
        prop_assert!(result.is_ok());
    }
}
