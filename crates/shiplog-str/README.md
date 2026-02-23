# shiplog-str

String utilities for shiplog.

## Overview

This crate provides string manipulation utilities for the shiplog ecosystem.

## Features

- String trimming and whitespace handling
- Case conversion (title case, snake_case, kebab-case)
- String padding
- Word counting
- String reversal

## Usage

```rust
use shiplog_str::{to_title_case, to_snake_case};

let title = to_title_case("hello world");
assert_eq!(title, "Hello World");

let snake = to_snake_case("HelloWorld");
assert_eq!(snake, "hello_world");
```
