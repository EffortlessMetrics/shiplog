use shiplog_str::*;

#[test]
fn trim_removes_whitespace() {
    assert_eq!(trim("  hello  "), "hello");
    assert_eq!(trim("\t\nhello\n\t"), "hello");
    assert_eq!(trim(""), "");
}

#[test]
fn to_title_case_basic() {
    assert_eq!(to_title_case("hello world"), "Hello World");
    assert_eq!(to_title_case("one-two"), "One-Two");
    assert_eq!(to_title_case("snake_case"), "Snake_Case");
}

#[test]
fn to_snake_case_basic() {
    assert_eq!(to_snake_case("HelloWorld"), "hello_world");
    assert_eq!(to_snake_case("hello-world"), "hello_world");
    assert_eq!(to_snake_case("Hello World"), "hello_world");
}

#[test]
fn to_kebab_case_basic() {
    assert_eq!(to_kebab_case("HelloWorld"), "hello-world");
    assert_eq!(to_kebab_case("hello_world"), "hello-world");
    assert_eq!(to_kebab_case("Hello World"), "hello-world");
}

#[test]
fn is_blank_edge_cases() {
    assert!(is_blank(""));
    assert!(is_blank("   "));
    assert!(is_blank("\t\n"));
    assert!(!is_blank("x"));
    assert!(!is_blank(" x "));
}

#[test]
fn reverse_basic() {
    assert_eq!(reverse("hello"), "olleh");
    assert_eq!(reverse(""), "");
    assert_eq!(reverse("a"), "a");
    assert_eq!(reverse("ab"), "ba");
}

#[test]
fn word_count_edge_cases() {
    assert_eq!(word_count(""), 0);
    assert_eq!(word_count("one"), 1);
    assert_eq!(word_count("   "), 0);
    assert_eq!(word_count("  one  two  three  "), 3);
}

#[test]
fn remove_whitespace_basic() {
    assert_eq!(remove_whitespace("a b c"), "abc");
    assert_eq!(remove_whitespace(""), "");
    assert_eq!(remove_whitespace("  "), "");
}

#[test]
fn pad_left_basic() {
    assert_eq!(pad_left("hi", 5, '0'), "000hi");
    assert_eq!(pad_left("hello", 3, '0'), "hello");
    assert_eq!(pad_left("", 3, 'x'), "xxx");
}

#[test]
fn pad_right_basic() {
    assert_eq!(pad_right("hi", 5, '0'), "hi000");
    assert_eq!(pad_right("hello", 3, '0'), "hello");
    assert_eq!(pad_right("", 3, 'x'), "xxx");
}
