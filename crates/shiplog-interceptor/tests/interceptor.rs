use shiplog_interceptor::{
    Interceptor, InterceptorBuilder, InterceptorChain, InterceptorConfig, RequestContext,
    RequestInterceptor, ResponseContext,
};

// --- RequestContext tests ---

#[test]
fn request_context_basic() {
    let ctx = RequestContext::new("GET", "/api/v1/users");
    assert_eq!(ctx.method, "GET");
    assert_eq!(ctx.path, "/api/v1/users");
    assert!(ctx.headers.is_empty());
}

#[test]
fn request_context_with_headers() {
    let ctx = RequestContext::new("POST", "/data")
        .with_header("Content-Type", "application/json")
        .with_header("Authorization", "Bearer abc")
        .with_header("X-Request-Id", "123");
    assert_eq!(ctx.headers.len(), 3);
    assert_eq!(ctx.headers[0].0, "Content-Type");
    assert_eq!(ctx.headers[2].1, "123");
}

// --- ResponseContext tests ---

#[test]
fn response_context_basic() {
    let ctx = ResponseContext::new(200);
    assert_eq!(ctx.status, 200);
    assert!(ctx.body.is_none());
    assert!(ctx.headers.is_empty());
}

#[test]
fn response_context_with_body_and_headers() {
    let ctx = ResponseContext::new(404)
        .with_body("Not Found")
        .with_header("Content-Type", "text/plain");
    assert_eq!(ctx.status, 404);
    assert_eq!(ctx.body, Some("Not Found".to_string()));
    assert_eq!(ctx.headers.len(), 1);
}

#[test]
fn response_context_status_codes() {
    for code in [200u16, 201, 301, 400, 401, 403, 404, 500] {
        let ctx = ResponseContext::new(code);
        assert_eq!(ctx.status, code);
    }
}

// --- RequestInterceptor tests ---

#[test]
fn request_interceptor_transforms_input() {
    let interceptor = RequestInterceptor::new(|ctx: &RequestContext, req: &String| {
        format!("[{}] {} {}", ctx.method, ctx.path, req)
    });
    let ctx = RequestContext::new("GET", "/test");
    let result = interceptor.intercept(&ctx, &"payload".to_string());
    assert_eq!(result, "[GET] /test payload");
}

#[test]
fn request_interceptor_uses_headers() {
    let interceptor = RequestInterceptor::new(|ctx: &RequestContext, _req: &i32| -> String {
        ctx.headers
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ")
    });
    let ctx = RequestContext::new("POST", "/").with_header("X-Key", "val");
    let result = interceptor.intercept(&ctx, &42);
    assert_eq!(result, "X-Key=val");
}

// --- InterceptorChain tests ---

struct PassthroughInterceptor {
    prefix: String,
}

impl PassthroughInterceptor {
    fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }
}

impl Interceptor<String, String> for PassthroughInterceptor {
    fn intercept(&self, _context: &RequestContext, request: &String) -> Option<String> {
        Some(format!("{}: {}", self.prefix, request))
    }
    fn on_response(&self, _context: &ResponseContext, _response: &String) {}
}

struct FilterInterceptor {
    allowed_method: String,
}

impl FilterInterceptor {
    fn new(method: &str) -> Self {
        Self {
            allowed_method: method.to_string(),
        }
    }
}

impl Interceptor<String, String> for FilterInterceptor {
    fn intercept(&self, context: &RequestContext, request: &String) -> Option<String> {
        if context.method == self.allowed_method {
            Some(request.clone())
        } else {
            None
        }
    }
    fn on_response(&self, _context: &ResponseContext, _response: &String) {}
}

#[test]
fn chain_empty_returns_none() {
    let chain: InterceptorChain<String, String> = InterceptorChain::new();
    let ctx = RequestContext::new("GET", "/test");
    assert!(chain.intercept(&ctx, &"data".to_string()).is_none());
}

#[test]
fn chain_default_returns_none() {
    let chain: InterceptorChain<String, String> = InterceptorChain::default();
    let ctx = RequestContext::new("GET", "/test");
    assert!(chain.intercept(&ctx, &"data".to_string()).is_none());
}

#[test]
fn chain_single_interceptor() {
    let chain = InterceptorChain::new().with(PassthroughInterceptor::new("handler"));
    let ctx = RequestContext::new("GET", "/");
    let result = chain.intercept(&ctx, &"input".to_string());
    assert_eq!(result, Some("handler: input".to_string()));
}

#[test]
fn chain_returns_first_match() {
    let chain = InterceptorChain::new()
        .with(PassthroughInterceptor::new("first"))
        .with(PassthroughInterceptor::new("second"));
    let ctx = RequestContext::new("GET", "/");
    let result = chain.intercept(&ctx, &"data".to_string());
    assert_eq!(result, Some("first: data".to_string()));
}

#[test]
fn chain_skips_non_matching_interceptors() {
    let chain = InterceptorChain::new()
        .with(FilterInterceptor::new("POST"))
        .with(PassthroughInterceptor::new("fallback"));
    let ctx = RequestContext::new("GET", "/");
    let result = chain.intercept(&ctx, &"data".to_string());
    assert_eq!(result, Some("fallback: data".to_string()));
}

#[test]
fn chain_all_skip_returns_none() {
    let chain = InterceptorChain::new()
        .with(FilterInterceptor::new("POST"))
        .with(FilterInterceptor::new("PUT"));
    let ctx = RequestContext::new("GET", "/");
    assert!(chain.intercept(&ctx, &"data".to_string()).is_none());
}

// --- InterceptorBuilder / Config tests ---

#[test]
fn builder_defaults() {
    let config = InterceptorBuilder::new().build();
    assert_eq!(config.name, "interceptor");
    assert!(config.enabled);
}

#[test]
fn builder_custom() {
    let config = InterceptorBuilder::new()
        .name("auth")
        .enabled(false)
        .build();
    assert_eq!(config.name, "auth");
    assert!(!config.enabled);
}

#[test]
fn builder_default_trait() {
    let config = InterceptorBuilder::default().build();
    assert_eq!(config.name, "interceptor");
}

#[test]
fn config_display() {
    let config = InterceptorConfig::new("my-int");
    assert_eq!(config.to_string(), "my-int (enabled: true)");
}

#[test]
fn config_clone_is_independent() {
    let config = InterceptorConfig::new("orig");
    let mut cloned = config.clone();
    cloned.name = "cloned".to_string();
    assert_eq!(config.name, "orig");
}

// --- Property tests ---

mod proptests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn request_context_preserves_method_and_path(
            method in "(GET|POST|PUT|DELETE|PATCH)",
            path in "/[a-z/]{0,50}",
        ) {
            let ctx = RequestContext::new(&method, &path);
            prop_assert_eq!(&ctx.method, &method);
            prop_assert_eq!(&ctx.path, &path);
        }

        #[test]
        fn response_context_preserves_status(status in 100u16..600) {
            let ctx = ResponseContext::new(status);
            prop_assert_eq!(ctx.status, status);
        }

        #[test]
        fn interceptor_config_name_preserved(name in "[a-zA-Z\\-]{1,30}") {
            let config = InterceptorConfig::new(&name);
            prop_assert_eq!(&config.name, &name);
            prop_assert!(config.enabled);
        }
    }
}
