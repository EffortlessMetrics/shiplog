use shiplog_template::*;

#[test]
fn render_variable_substitution() {
    let engine = TemplateEngine::new();
    let mut ctx = TemplateContext::new();
    ctx.set("name", "Alice");
    let result = engine.render("Hello, {{ name }}!", &ctx).unwrap();
    assert_eq!(result, "Hello, Alice!");
}

#[test]
fn render_missing_variable() {
    let engine = TemplateEngine::new();
    let ctx = TemplateContext::new();
    let result = engine.render("Hello, {{ name }}!", &ctx).unwrap();
    assert_eq!(result, "Hello, !");
}

#[test]
fn render_no_variables() {
    let engine = TemplateEngine::new();
    let ctx = TemplateContext::new();
    let result = engine.render("plain text", &ctx).unwrap();
    assert_eq!(result, "plain text");
}

#[test]
fn render_empty_template() {
    let engine = TemplateEngine::new();
    let ctx = TemplateContext::new();
    assert_eq!(engine.render("", &ctx).unwrap(), "");
}

#[test]
fn render_multiple_variables() {
    let engine = TemplateEngine::new();
    let mut ctx = TemplateContext::new();
    ctx.set("a", "X");
    ctx.set("b", "Y");
    let result = engine.render("{{ a }}+{{ b }}", &ctx).unwrap();
    assert_eq!(result, "X+Y");
}

#[test]
fn render_unclosed_variable_error() {
    let engine = TemplateEngine::new();
    let ctx = TemplateContext::new();
    assert!(engine.render("{{ unclosed", &ctx).is_err());
}

#[test]
fn custom_delimiters() {
    let engine = TemplateEngine::with_delimiters("<<", ">>", "<%", "%>");
    let mut ctx = TemplateContext::new();
    ctx.set("name", "Bob");
    let result = engine.render("Hi << name >>!", &ctx).unwrap();
    assert_eq!(result, "Hi Bob!");
}

#[test]
fn context_set_overwrite() {
    let mut ctx = TemplateContext::new();
    ctx.set("k", "v1");
    ctx.set("k", "v2");
    assert_eq!(ctx.get("k"), Some(&TemplateValue::String("v2".to_string())));
}

#[test]
fn context_is_truthy() {
    let mut ctx = TemplateContext::new();
    ctx.set("t", true);
    ctx.set("f", false);
    ctx.set("s", "hi");
    ctx.set("e", "");
    assert!(ctx.is_truthy("t"));
    assert!(!ctx.is_truthy("f"));
    assert!(ctx.is_truthy("s"));
    assert!(!ctx.is_truthy("e"));
    assert!(!ctx.is_truthy("missing"));
}

#[test]
fn template_value_types_via_context() {
    let mut ctx = TemplateContext::new();
    ctx.set("bool_true", true);
    ctx.set("bool_false", false);
    ctx.set("num_one", 1i64);
    ctx.set("num_zero", 0i64);
    assert!(ctx.is_truthy("bool_true"));
    assert!(!ctx.is_truthy("bool_false"));
    assert!(ctx.is_truthy("num_one"));
    assert!(!ctx.is_truthy("num_zero"));
}

#[test]
fn template_value_display() {
    assert_eq!(TemplateValue::String("hi".to_string()).to_string(), "hi");
    assert_eq!(TemplateValue::Number(42).to_string(), "42");
    assert_eq!(TemplateValue::Boolean(true).to_string(), "true");
    assert_eq!(TemplateValue::Null.to_string(), "");
}

#[test]
fn template_value_from_conversions() {
    let _: TemplateValue = "hello".into();
    let _: TemplateValue = String::from("hello").into();
    let _: TemplateValue = 42i64.into();
    let _: TemplateValue = 1.5f64.into();
    let _: TemplateValue = true.into();
}

#[test]
fn engine_default() {
    let engine = TemplateEngine::default();
    let ctx = TemplateContext::new();
    assert_eq!(engine.render("text", &ctx).unwrap(), "text");
}
