//! Middleware chain pattern utilities for shiplog.
//!
//! This crate provides utilities for building and managing middleware chains.

use std::fmt;

/// Middleware trait for request/response processing
pub trait Middleware<T>: Send + Sync {
    fn process(
        &self,
        context: &mut MiddlewareContext,
        next: &mut dyn FnMut(&mut MiddlewareContext),
    );
}

/// Context passed through middleware chain
#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    pub request_id: String,
    pub data: Vec<(String, String)>,
    pub processed: bool,
}

impl MiddlewareContext {
    pub fn new(request_id: &str) -> Self {
        Self {
            request_id: request_id.to_string(),
            data: Vec::new(),
            processed: false,
        }
    }

    pub fn with_data(mut self, key: &str, value: &str) -> Self {
        self.data.push((key.to_string(), value.to_string()));
        self
    }

    pub fn get_data(&self, key: &str) -> Option<&String> {
        self.data.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

/// Middleware chain for processing requests
pub struct MiddlewareChain<T> {
    middlewares: Vec<Box<dyn Middleware<T>>>,
}

impl<T> MiddlewareChain<T> {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn with<M>(mut self, middleware: M) -> Self
    where
        M: Middleware<T> + 'static,
    {
        self.middlewares.push(Box::new(middleware));
        self
    }

    pub fn execute(&mut self, context: &mut MiddlewareContext) {
        // Create a boxed function that will be called recursively
        let _middlewares = &mut self.middlewares;

        fn execute_next<T>(
            middlewares: &Vec<Box<dyn Middleware<T>>>,
            index: usize,
            context: &mut MiddlewareContext,
        ) {
            if index < middlewares.len() {
                let middleware = &middlewares[index];
                let mut next = |ctx: &mut MiddlewareContext| {
                    execute_next(middlewares, index + 1, ctx);
                };
                middleware.process(context, &mut next);
            }
        }

        execute_next(&self.middlewares, 0, context);
        context.processed = true;
    }
}

impl<T> Default for MiddlewareChain<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating middleware configurations
#[derive(Debug)]
pub struct MiddlewareBuilder {
    name: String,
    enabled: bool,
    order: i32,
}

impl MiddlewareBuilder {
    pub fn new() -> Self {
        Self {
            name: "middleware".to_string(),
            enabled: true,
            order: 0,
        }
    }

    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    pub fn build(self) -> MiddlewareConfig {
        MiddlewareConfig {
            name: self.name,
            enabled: self.enabled,
            order: self.order,
        }
    }
}

impl Default for MiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for middleware
#[derive(Debug, Clone)]
pub struct MiddlewareConfig {
    pub name: String,
    pub enabled: bool,
    pub order: i32,
}

impl MiddlewareConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
            order: 0,
        }
    }
}

impl fmt::Display for MiddlewareConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} (order: {}, enabled: {})",
            self.name, self.order, self.enabled
        )
    }
}

/// Simple logging middleware
pub struct LoggingMiddleware {
    name: String,
}

impl LoggingMiddleware {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl<T> Middleware<T> for LoggingMiddleware {
    fn process(
        &self,
        context: &mut MiddlewareContext,
        next: &mut dyn FnMut(&mut MiddlewareContext),
    ) {
        // Pre-processing
        context
            .data
            .push((format!("{}-pre", self.name), "executed".to_string()));

        // Call next middleware
        next(context);

        // Post-processing
        context
            .data
            .push((format!("{}-post", self.name), "executed".to_string()));
    }
}

/// Timing middleware for measuring execution time
pub struct TimingMiddleware {
    #[allow(dead_code)]
    name: String,
}

impl TimingMiddleware {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl<T> Middleware<T> for TimingMiddleware {
    fn process(
        &self,
        _context: &mut MiddlewareContext,
        next: &mut dyn FnMut(&mut MiddlewareContext),
    ) {
        next(_context);
        // Timing info would be added here in a real implementation
    }
}

/// Validation middleware
pub struct ValidationMiddleware {
    name: String,
}

impl ValidationMiddleware {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

impl<T> Middleware<T> for ValidationMiddleware {
    fn process(
        &self,
        context: &mut MiddlewareContext,
        next: &mut dyn FnMut(&mut MiddlewareContext),
    ) {
        context
            .data
            .push((format!("{}-validated", self.name), "true".to_string()));
        next(context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_context_new() {
        let ctx = MiddlewareContext::new("req-123");
        assert_eq!(ctx.request_id, "req-123");
        assert!(ctx.data.is_empty());
        assert!(!ctx.processed);
    }

    #[test]
    fn test_middleware_context_with_data() {
        let ctx = MiddlewareContext::new("req-123")
            .with_data("key1", "value1")
            .with_data("key2", "value2");

        assert_eq!(ctx.data.len(), 2);
        assert_eq!(ctx.get_data("key1"), Some(&"value1".to_string()));
        assert_eq!(ctx.get_data("key2"), Some(&"value2".to_string()));
        assert_eq!(ctx.get_data("nonexistent"), None);
    }

    #[test]
    fn test_middleware_builder() {
        let config = MiddlewareBuilder::new()
            .name("test-middleware")
            .enabled(true)
            .order(5)
            .build();

        assert_eq!(config.name, "test-middleware");
        assert!(config.enabled);
        assert_eq!(config.order, 5);
    }

    #[test]
    fn test_middleware_config_display() {
        let config = MiddlewareConfig::new("my-middleware");
        assert_eq!(
            config.to_string(),
            "my-middleware (order: 0, enabled: true)"
        );
    }

    #[test]
    fn test_middleware_chain_empty() {
        let mut chain: MiddlewareChain<String> = MiddlewareChain::new();
        let mut ctx = MiddlewareContext::new("req-1");

        chain.execute(&mut ctx);

        assert!(ctx.processed);
    }

    #[test]
    fn test_middleware_chain_with_logging() {
        let mut chain: MiddlewareChain<String> =
            MiddlewareChain::new().with(LoggingMiddleware::new("logger"));

        let mut ctx = MiddlewareContext::new("req-1");
        chain.execute(&mut ctx);

        assert!(ctx.processed);
        assert!(
            ctx.data
                .iter()
                .any(|(k, v)| k == "logger-pre" && v == "executed")
        );
        assert!(
            ctx.data
                .iter()
                .any(|(k, v)| k == "logger-post" && v == "executed")
        );
    }

    #[test]
    fn test_middleware_chain_multiple() {
        let mut chain: MiddlewareChain<String> = MiddlewareChain::new()
            .with(LoggingMiddleware::new("logger"))
            .with(ValidationMiddleware::new("validator"))
            .with(TimingMiddleware::new("timer"));

        let mut ctx = MiddlewareContext::new("req-1");
        chain.execute(&mut ctx);

        assert!(ctx.processed);
        assert!(ctx.data.iter().any(|(k, _)| k == "logger-pre"));
        assert!(ctx.data.iter().any(|(k, _)| k == "validator-validated"));
    }

    #[test]
    fn test_timing_middleware() {
        let middleware: LoggingMiddleware = LoggingMiddleware::new("timer");
        let mut ctx = MiddlewareContext::new("req-1");

        let mut next = |c: &mut MiddlewareContext| {
            c.data.push(("next-called".to_string(), "true".to_string()));
        };

        <LoggingMiddleware as Middleware<String>>::process(&middleware, &mut ctx, &mut next);

        assert!(
            ctx.data
                .iter()
                .any(|(k, v)| k == "next-called" && v == "true")
        );
    }

    #[test]
    fn test_validation_middleware() {
        let middleware: LoggingMiddleware = LoggingMiddleware::new("validator");
        let mut ctx = MiddlewareContext::new("req-1");

        let mut next = |_: &mut MiddlewareContext| {};

        <LoggingMiddleware as Middleware<String>>::process(&middleware, &mut ctx, &mut next);

        assert!(ctx.data.iter().any(|(k, _)| k == "validator-pre"));
    }
}
