use shiplog_normalizer::*;

#[test]
fn normalize_json_key_camel_case() {
    assert_eq!(normalize_json_key("camelCase"), "camel_case");
}

#[test]
fn normalize_json_key_pascal_case() {
    assert_eq!(normalize_json_key("PascalCase"), "pascal_case");
}

#[test]
fn normalize_json_key_snake_case_passthrough() {
    assert_eq!(normalize_json_key("snake_case"), "snake_case");
}

#[test]
fn normalize_json_value_trims() {
    assert_eq!(normalize_json_value("  test  "), "test");
    assert_eq!(normalize_json_value("\nvalue\n"), "value");
}

#[test]
fn normalize_yaml_key_same_as_json() {
    let key = "camelCase";
    assert_eq!(normalize_yaml_key(key), normalize_json_key(key));
}

#[test]
fn normalize_slug_basic() {
    assert_eq!(normalize_slug("Hello World"), "hello-world");
    assert_eq!(normalize_slug("Test__Name"), "test-name");
    assert_eq!(normalize_slug("123 ABC"), "123-abc");
    assert_eq!(normalize_slug(""), "");
}

#[test]
fn normalize_slug_trailing_special() {
    assert_eq!(normalize_slug("hello "), "hello");
    assert_eq!(normalize_slug(" hello"), "hello");
}

#[test]
fn normalize_version_basic() {
    assert_eq!(normalize_version("1.0.0"), "1.0.0");
    assert_eq!(normalize_version("v2.5"), "2.5.0");
    assert_eq!(normalize_version("3"), "3.0.0");
    assert_eq!(normalize_version("1.2.3.4"), "1.2.3");
}

#[test]
fn normalize_semver_range_caret() {
    assert_eq!(normalize_semver_range("^1.0.0"), "^1.0.0");
    assert_eq!(normalize_semver_range("^2"), "^2.0.0");
}

#[test]
fn normalize_semver_range_tilde() {
    assert_eq!(normalize_semver_range("~2.0"), "~2.0.0");
}

#[test]
fn normalize_csv_header_basic() {
    assert_eq!(normalize_csv_header("First Name"), "first_name");
    assert_eq!(normalize_csv_header("LAST_NAME"), "last_name");
    assert_eq!(normalize_csv_header("  Email  "), "email");
}

#[test]
fn normalize_phone_10_digits() {
    assert_eq!(normalize_phone("1234567890"), "+11234567890");
}

#[test]
fn normalize_phone_with_formatting() {
    assert_eq!(normalize_phone("+1 234 567 8901"), "+12345678901");
}

#[test]
fn normalize_date_trims() {
    assert_eq!(normalize_date("  2025-01-01  "), "2025-01-01");
}

#[test]
fn string_normalizer_default() {
    let n = StringNormalizer::default_config();
    assert_eq!(n.normalize("  HELLO  "), "hello");
}

#[test]
fn string_normalizer_custom() {
    let config = NormalizerConfig {
        lowercase: true,
        trim: true,
        remove_special: true,
    };
    let n = StringNormalizer::new(config);
    assert_eq!(n.normalize("Hello!@#$World"), "hello world");
}

#[test]
fn normalizer_config_defaults() {
    let config = NormalizerConfig::default();
    assert!(config.lowercase);
    assert!(config.trim);
    assert!(!config.remove_special);
}
