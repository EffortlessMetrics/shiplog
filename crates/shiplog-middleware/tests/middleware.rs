use shiplog_middleware::{
    LoggingMiddleware, MiddlewareBuilder, MiddlewareChain, MiddlewareConfig, MiddlewareContext,
    TimingMiddleware, ValidationMiddleware,
};

// --- MiddlewareContext tests ---

#[test]
fn context_new_defaults() {
    let ctx = MiddlewareContext::new("req-1");
    assert_eq!(ctx.request_id, "req-1");
    assert!(ctx.data.is_empty());
    assert!(!ctx.processed);
}

#[test]
fn context_with_data_chaining() {
    let ctx = MiddlewareContext::new("r")
        .with_data("a", "1")
        .with_data("b", "2")
        .with_data("c", "3");
    assert_eq!(ctx.data.len(), 3);
    assert_eq!(ctx.get_data("a"), Some(&"1".to_string()));
    assert_eq!(ctx.get_data("c"), Some(&"3".to_string()));
}

#[test]
fn context_get_data_missing_key_returns_none() {
    let ctx = MiddlewareContext::new("r").with_data("a", "1");
    assert_eq!(ctx.get_data("missing"), None);
}

#[test]
fn context_duplicate_keys_returns_first() {
    let ctx = MiddlewareContext::new("r")
        .with_data("k", "first")
        .with_data("k", "second");
    // `find` returns first match
    assert_eq!(ctx.get_data("k"), Some(&"first".to_string()));
}

// --- MiddlewareChain tests ---

#[test]
fn empty_chain_marks_processed() {
    let mut chain: MiddlewareChain<String> = MiddlewareChain::new();
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);
    assert!(ctx.processed);
}

#[test]
fn chain_default_is_empty() {
    let mut chain: MiddlewareChain<String> = MiddlewareChain::default();
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);
    assert!(ctx.processed);
    assert!(ctx.data.is_empty());
}

#[test]
fn single_logging_middleware_adds_pre_and_post() {
    let mut chain: MiddlewareChain<String> =
        MiddlewareChain::new().with(LoggingMiddleware::new("log"));
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);

    assert!(ctx.data.iter().any(|(k, _)| k == "log-pre"));
    assert!(ctx.data.iter().any(|(k, _)| k == "log-post"));
}

#[test]
fn middleware_chain_ordering() {
    let mut chain: MiddlewareChain<String> = MiddlewareChain::new()
        .with(LoggingMiddleware::new("first"))
        .with(LoggingMiddleware::new("second"))
        .with(LoggingMiddleware::new("third"));

    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);

    // Pre-processing: first, second, third (in order)
    // Post-processing: third, second, first (reverse)
    let keys: Vec<&str> = ctx.data.iter().map(|(k, _)| k.as_str()).collect();
    let first_pre = keys.iter().position(|k| *k == "first-pre").unwrap();
    let second_pre = keys.iter().position(|k| *k == "second-pre").unwrap();
    let third_pre = keys.iter().position(|k| *k == "third-pre").unwrap();
    let third_post = keys.iter().position(|k| *k == "third-post").unwrap();
    let second_post = keys.iter().position(|k| *k == "second-post").unwrap();
    let first_post = keys.iter().position(|k| *k == "first-post").unwrap();

    assert!(first_pre < second_pre);
    assert!(second_pre < third_pre);
    assert!(third_pre < third_post);
    assert!(third_post < second_post);
    assert!(second_post < first_post);
}

#[test]
fn validation_middleware_adds_validated_entry() {
    let mut chain: MiddlewareChain<String> =
        MiddlewareChain::new().with(ValidationMiddleware::new("val"));
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);

    assert!(
        ctx.data
            .iter()
            .any(|(k, v)| k == "val-validated" && v == "true")
    );
}

#[test]
fn timing_middleware_calls_next() {
    let mut chain: MiddlewareChain<String> = MiddlewareChain::new()
        .with(TimingMiddleware::new("timer"))
        .with(LoggingMiddleware::new("inner"));
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);

    // inner should have been called
    assert!(ctx.data.iter().any(|(k, _)| k == "inner-pre"));
}

#[test]
fn multiple_middleware_types_compose() {
    let mut chain: MiddlewareChain<String> = MiddlewareChain::new()
        .with(LoggingMiddleware::new("log"))
        .with(ValidationMiddleware::new("val"))
        .with(TimingMiddleware::new("time"));
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);

    assert!(ctx.processed);
    assert!(ctx.data.iter().any(|(k, _)| k == "log-pre"));
    assert!(ctx.data.iter().any(|(k, _)| k == "val-validated"));
}

// --- MiddlewareBuilder tests ---

#[test]
fn builder_defaults() {
    let config = MiddlewareBuilder::new().build();
    assert_eq!(config.name, "middleware");
    assert!(config.enabled);
    assert_eq!(config.order, 0);
}

#[test]
fn builder_custom_values() {
    let config = MiddlewareBuilder::new()
        .name("custom")
        .enabled(false)
        .order(99)
        .build();
    assert_eq!(config.name, "custom");
    assert!(!config.enabled);
    assert_eq!(config.order, 99);
}

#[test]
fn builder_default_trait() {
    let config = MiddlewareBuilder::default().build();
    assert_eq!(config.name, "middleware");
}

// --- MiddlewareConfig tests ---

#[test]
fn config_new_defaults() {
    let config = MiddlewareConfig::new("test");
    assert_eq!(config.name, "test");
    assert!(config.enabled);
    assert_eq!(config.order, 0);
}

#[test]
fn config_display() {
    let config = MiddlewareConfig {
        name: "auth".to_string(),
        enabled: false,
        order: 5,
    };
    assert_eq!(config.to_string(), "auth (order: 5, enabled: false)");
}

#[test]
fn config_clone_is_independent() {
    let config = MiddlewareConfig::new("orig");
    let mut cloned = config.clone();
    cloned.name = "cloned".to_string();
    assert_eq!(config.name, "orig");
}

// --- Edge cases ---

#[test]
fn chain_execute_twice() {
    let mut chain: MiddlewareChain<String> =
        MiddlewareChain::new().with(LoggingMiddleware::new("log"));
    let mut ctx = MiddlewareContext::new("req");
    chain.execute(&mut ctx);
    let count_after_first = ctx.data.len();

    chain.execute(&mut ctx);
    assert!(ctx.data.len() > count_after_first);
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn context_preserves_request_id(id in "[a-zA-Z0-9\\-]{1,50}") {
            let ctx = MiddlewareContext::new(&id);
            prop_assert_eq!(&ctx.request_id, &id);
        }

        #[test]
        fn context_with_data_preserves_all(
            pairs in proptest::collection::vec(("[a-z]{1,10}", "[a-z]{1,10}"), 0..20)
        ) {
            let mut ctx = MiddlewareContext::new("r");
            for (k, v) in &pairs {
                ctx = ctx.with_data(k, v);
            }
            prop_assert_eq!(ctx.data.len(), pairs.len());
        }

        #[test]
        fn builder_order_preserved(order in -1000i32..1000) {
            let config = MiddlewareBuilder::new().order(order).build();
            prop_assert_eq!(config.order, order);
        }
    }
}
