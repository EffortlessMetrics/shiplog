//! Request/response interception utilities for shiplog.
//!
//! This crate provides utilities for intercepting and modifying requests and responses.

use std::fmt;
use std::marker::PhantomData;

/// Request interceptor context
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
}

impl RequestContext {
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: Vec::new(),
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }
}

/// Response interceptor context
#[derive(Debug, Clone)]
pub struct ResponseContext {
    pub status: u16,
    pub body: Option<String>,
    pub headers: Vec<(String, String)>,
}

impl ResponseContext {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            body: None,
            headers: Vec::new(),
        }
    }

    pub fn with_body(mut self, body: &str) -> Self {
        self.body = Some(body.to_string());
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }
}

/// Interceptor trait for request/response interception
pub trait Interceptor<T, R>: Send + Sync {
    fn intercept(&self, context: &RequestContext, request: &T) -> Option<R>;
    fn on_response(&self, context: &ResponseContext, response: &R);
}

/// Basic request interceptor
pub struct RequestInterceptor<T, R, F> {
    handler: F,
    _phantom: std::marker::PhantomData<fn() -> (T, R)>,
}

impl<T, R, F> RequestInterceptor<T, R, F>
where
    F: Fn(&RequestContext, &T) -> R + Send + Sync,
{
    pub fn new(handler: F) -> Self {
        Self {
            handler,
            _phantom: PhantomData,
        }
    }

    pub fn intercept(&self, context: &RequestContext, request: &T) -> R {
        (self.handler)(context, request)
    }
}

/// Interceptor chain for chaining multiple interceptors
pub struct InterceptorChain<T, R> {
    interceptors: Vec<Box<dyn Interceptor<T, R>>>,
}

impl<T, R> InterceptorChain<T, R> {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    pub fn with<I>(mut self, interceptor: I) -> Self
    where
        I: Interceptor<T, R> + 'static,
    {
        self.interceptors.push(Box::new(interceptor));
        self
    }

    pub fn intercept(&self, context: &RequestContext, request: &T) -> Option<R> {
        for interceptor in &self.interceptors {
            if let Some(result) = interceptor.intercept(context, request) {
                return Some(result);
            }
        }
        None
    }
}

impl<T, R> Default for InterceptorChain<T, R> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating interceptors
#[derive(Debug)]
pub struct InterceptorBuilder {
    name: String,
    enabled: bool,
}

impl InterceptorBuilder {
    pub fn new() -> Self {
        Self {
            name: "interceptor".to_string(),
            enabled: true,
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

    pub fn build(self) -> InterceptorConfig {
        InterceptorConfig {
            name: self.name,
            enabled: self.enabled,
        }
    }
}

impl Default for InterceptorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for an interceptor
#[derive(Debug, Clone)]
pub struct InterceptorConfig {
    pub name: String,
    pub enabled: bool,
}

impl InterceptorConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            enabled: true,
        }
    }
}

impl fmt::Display for InterceptorConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (enabled: {})", self.name, self.enabled)
    }
}

/// Test interceptor implementation
#[cfg(test)]
struct TestInterceptor {
    name: String,
}

#[cfg(test)]
impl TestInterceptor {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[cfg(test)]
impl Interceptor<String, String> for TestInterceptor {
    fn intercept(&self, _context: &RequestContext, request: &String) -> Option<String> {
        Some(format!("{}: {}", self.name, request))
    }

    fn on_response(&self, _context: &ResponseContext, _response: &String) {
        // No-op for testing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_context_new() {
        let ctx = RequestContext::new("GET", "/api/test");
        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/test");
        assert!(ctx.headers.is_empty());
    }

    #[test]
    fn test_request_context_with_header() {
        let ctx = RequestContext::new("POST", "/api/data")
            .with_header("Content-Type", "application/json")
            .with_header("Authorization", "Bearer token");

        assert_eq!(ctx.headers.len(), 2);
        assert_eq!(
            ctx.headers[0],
            ("Content-Type".to_string(), "application/json".to_string())
        );
    }

    #[test]
    fn test_response_context_new() {
        let ctx = ResponseContext::new(200);
        assert_eq!(ctx.status, 200);
        assert!(ctx.body.is_none());
    }

    #[test]
    fn test_response_context_with_body() {
        let ctx = ResponseContext::new(200).with_body("Hello, World!");
        assert_eq!(ctx.body, Some("Hello, World!".to_string()));
    }

    #[test]
    fn test_interceptor_builder() {
        let config = InterceptorBuilder::new()
            .name("test-interceptor")
            .enabled(false)
            .build();

        assert_eq!(config.name, "test-interceptor");
        assert!(!config.enabled);
    }

    #[test]
    fn test_interceptor_config_display() {
        let config = InterceptorConfig::new("my-interceptor");
        assert_eq!(config.to_string(), "my-interceptor (enabled: true)");
    }

    #[test]
    fn test_request_interceptor() {
        let interceptor = RequestInterceptor::new(|ctx: &RequestContext, req: &String| {
            format!("{} {} -> {}", ctx.method, ctx.path, req)
        });

        let ctx = RequestContext::new("GET", "/test");
        let result = interceptor.intercept(&ctx, &"request-data".to_string());
        assert_eq!(result, "GET /test -> request-data");
    }

    #[test]
    fn test_interceptor_chain() {
        let chain: InterceptorChain<String, String> = InterceptorChain::new()
            .with(TestInterceptor::new("interceptor1"))
            .with(TestInterceptor::new("interceptor2"));

        let ctx = RequestContext::new("GET", "/test");
        let result = chain.intercept(&ctx, &"data".to_string());
        assert!(result.is_some());
    }

    #[test]
    fn test_interceptor_chain_empty() {
        let chain: InterceptorChain<String, String> = InterceptorChain::new();

        let ctx = RequestContext::new("GET", "/test");
        let result = chain.intercept(&ctx, &"data".to_string());
        assert!(result.is_none());
    }
}
