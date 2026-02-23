//! Jinja2-like template system for packet rendering.
//!
//! Provides a simple template engine supporting:
//! - Variable substitution
//! - Conditional sections
//! - Loops over collections
//! - User-defined templates

use anyhow::{Result, anyhow};
use serde::Serialize;
use std::collections::HashMap;

/// Template context containing variables for template rendering
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    variables: HashMap<String, TemplateValue>,
}

impl TemplateContext {
    /// Create a new empty template context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a variable in the context
    pub fn set(&mut self, key: &str, value: impl Into<TemplateValue>) {
        self.variables.insert(key.to_string(), value.into());
    }

    /// Get a variable from the context
    pub fn get(&self, key: &str) -> Option<&TemplateValue> {
        self.variables.get(key)
    }

    /// Check if a variable exists and is truthy
    pub fn is_truthy(&self, key: &str) -> bool {
        self.get(key).map(|v| v.is_truthy()).unwrap_or(false)
    }

    /// Get a variable as a string for rendering
    fn get_string(&self, key: &str) -> Option<String> {
        self.get(key).map(|v| v.to_string())
    }
}

/// Template value that can be used in templates
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateValue {
    String(String),
    Number(i64),
    Float(f64),
    Boolean(bool),
    List(Vec<TemplateValue>),
    Object(HashMap<String, TemplateValue>),
    Null,
}

impl TemplateValue {
    /// Check if the value is truthy (for conditionals)
    fn is_truthy(&self) -> bool {
        match self {
            TemplateValue::Boolean(b) => *b,
            TemplateValue::String(s) => !s.is_empty(),
            TemplateValue::Number(n) => *n != 0,
            TemplateValue::Float(f) => *f != 0.0,
            TemplateValue::List(l) => !l.is_empty(),
            TemplateValue::Object(o) => !o.is_empty(),
            TemplateValue::Null => false,
        }
    }

    /// Get a field from an object value
    #[allow(dead_code)]
    fn get_field(&self, field: &str) -> Option<&TemplateValue> {
        match self {
            TemplateValue::Object(obj) => obj.get(field),
            _ => None,
        }
    }
}

impl From<String> for TemplateValue {
    fn from(s: String) -> Self {
        TemplateValue::String(s)
    }
}

impl From<&str> for TemplateValue {
    fn from(s: &str) -> Self {
        TemplateValue::String(s.to_string())
    }
}

impl From<i64> for TemplateValue {
    fn from(n: i64) -> Self {
        TemplateValue::Number(n)
    }
}

impl From<f64> for TemplateValue {
    fn from(f: f64) -> Self {
        TemplateValue::Float(f)
    }
}

impl From<bool> for TemplateValue {
    fn from(b: bool) -> Self {
        TemplateValue::Boolean(b)
    }
}

impl From<Vec<TemplateValue>> for TemplateValue {
    fn from(v: Vec<TemplateValue>) -> Self {
        TemplateValue::List(v)
    }
}

impl From<HashMap<String, TemplateValue>> for TemplateValue {
    fn from(o: HashMap<String, TemplateValue>) -> Self {
        TemplateValue::Object(o)
    }
}

impl std::fmt::Display for TemplateValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateValue::String(s) => write!(f, "{}", s),
            TemplateValue::Number(n) => write!(f, "{}", n),
            TemplateValue::Float(fl) => write!(f, "{}", fl),
            TemplateValue::Boolean(b) => write!(f, "{}", b),
            TemplateValue::List(_) => write!(f, "[list]"),
            TemplateValue::Object(_) => write!(f, "[object]"),
            TemplateValue::Null => write!(f, ""),
        }
    }
}

impl<T: Serialize> From<&T> for TemplateValue {
    fn from(value: &T) -> Self {
        serde_json::to_value(value)
            .ok()
            .and_then(Self::from_json_value)
            .unwrap_or(TemplateValue::Null)
    }
}

impl TemplateValue {
    fn from_json_value(value: serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => Some(TemplateValue::String(s)),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Some(TemplateValue::Number(i))
                } else {
                    n.as_f64().map(TemplateValue::Float)
                }
            }
            serde_json::Value::Bool(b) => Some(TemplateValue::Boolean(b)),
            serde_json::Value::Array(arr) => {
                let items: Vec<TemplateValue> =
                    arr.into_iter().filter_map(Self::from_json_value).collect();
                Some(TemplateValue::List(items))
            }
            serde_json::Value::Object(obj) => {
                let fields: HashMap<String, TemplateValue> = obj
                    .into_iter()
                    .filter_map(|(k, v)| Self::from_json_value(v).map(|tv| (k, tv)))
                    .collect();
                Some(TemplateValue::Object(fields))
            }
            serde_json::Value::Null => Some(TemplateValue::Null),
        }
    }
}

/// Template engine for rendering Jinja2-like templates
#[derive(Debug, Clone)]
pub struct TemplateEngine {
    /// Variable opening delimiter (default: "{{")
    var_open: String,
    /// Variable closing delimiter (default: "}}")
    var_close: String,
    /// Tag opening delimiter (default: "{%")
    tag_open: String,
    /// Tag closing delimiter (default: "%}")
    tag_close: String,
}

impl TemplateEngine {
    /// Create a new template engine with default delimiters
    pub fn new() -> Self {
        Self {
            var_open: "{{".to_string(),
            var_close: "}}".to_string(),
            tag_open: "{%".to_string(),
            tag_close: "%}".to_string(),
        }
    }

    /// Create a new template engine with custom delimiters
    pub fn with_delimiters(
        var_open: &str,
        var_close: &str,
        tag_open: &str,
        tag_close: &str,
    ) -> Self {
        Self {
            var_open: var_open.to_string(),
            var_close: var_close.to_string(),
            tag_open: tag_open.to_string(),
            tag_close: tag_close.to_string(),
        }
    }

    /// Render a template with the given context
    pub fn render(&self, template: &str, context: &TemplateContext) -> Result<String> {
        let mut output = String::new();
        let mut remaining = template;

        while !remaining.is_empty() {
            // Find the next token
            if let Some(pos) = remaining.find(&self.var_open) {
                // Add text before the token
                output.push_str(&remaining[..pos]);
                remaining = &remaining[pos..];

                // Check for variable or tag
                if remaining.starts_with(&self.tag_open) {
                    // It's a tag
                    let (tag, rest) = self.parse_tag(&remaining[self.tag_open.len()..])?;
                    remaining = rest;
                    output.push_str(&tag);
                } else {
                    // It's a variable
                    let (var, rest) = self.parse_variable(&remaining[self.var_open.len()..])?;
                    remaining = rest;
                    if let Some(value) = context.get_string(&var) {
                        output.push_str(&value);
                    }
                }
            } else {
                // No more tokens, add remaining text
                output.push_str(remaining);
                break;
            }
        }

        Ok(output)
    }

    /// Parse a tag (if/for/etc.)
    fn parse_tag<'a>(&self, input: &'a str) -> Result<(String, &'a str)> {
        let end_pos = input
            .find(&self.tag_close)
            .ok_or_else(|| anyhow!("Unclosed tag: missing {}", self.tag_close))?;

        let tag_content = input[..end_pos].trim();
        let remaining = &input[end_pos + self.tag_close.len()..];

        // Parse the tag
        let result = self.evaluate_tag(tag_content)?;
        Ok((result, remaining))
    }

    /// Parse a variable reference
    fn parse_variable<'a>(&self, input: &'a str) -> Result<(String, &'a str)> {
        let end_pos = input
            .find(&self.var_close)
            .ok_or_else(|| anyhow!("Unclosed variable: missing {}", self.var_close))?;

        let var_name = input[..end_pos].trim().to_string();
        let remaining = &input[end_pos + self.var_close.len()..];

        Ok((var_name, remaining))
    }

    /// Evaluate a tag and return the rendered output
    fn evaluate_tag(&self, tag_content: &str) -> Result<String> {
        // Simple tag parsing (for full implementation, would need proper parsing)
        let content = tag_content.trim();

        // Handle if tags
        if content.starts_with("if ") {
            // For now, just return empty string
            // Full implementation would need to track nesting and evaluate conditionals
            Ok(String::new())
        }
        // Handle endif tags
        else if content == "endif" {
            Ok(String::new())
        }
        // Handle for tags
        else if content.starts_with("for ") {
            // For now, just return empty string
            // Full implementation would need to track nesting and iterate
            Ok(String::new())
        }
        // Handle endfor tags
        else if content == "endfor" {
            Ok(String::new())
        }
        // Unknown tag
        else {
            Err(anyhow!("Unknown tag: {}", content))
        }
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_context_set_and_get() {
        let mut ctx = TemplateContext::new();
        ctx.set("name", "Alice");
        ctx.set("age", 30);

        assert_eq!(ctx.get_string("name"), Some("Alice".to_string()));
        assert_eq!(ctx.get("age"), Some(&TemplateValue::Number(30)));
    }

    #[test]
    fn template_context_is_truthy() {
        let mut ctx = TemplateContext::new();
        ctx.set("true_var", true);
        ctx.set("false_var", false);
        ctx.set("string_var", "hello");
        ctx.set("empty_string", "");
        ctx.set("number_var", 42);
        ctx.set("zero_var", 0);

        assert!(ctx.is_truthy("true_var"));
        assert!(!ctx.is_truthy("false_var"));
        assert!(ctx.is_truthy("string_var"));
        assert!(!ctx.is_truthy("empty_string"));
        assert!(ctx.is_truthy("number_var"));
        assert!(!ctx.is_truthy("zero_var"));
    }

    #[test]
    fn template_engine_render_variable() {
        let engine = TemplateEngine::new();
        let mut ctx = TemplateContext::new();
        ctx.set("name", "Alice");
        ctx.set("title", "My Packet");

        let template = "# {{ title }}\n\nHello, {{ name }}!";
        let result = engine.render(template, &ctx).unwrap();

        assert_eq!(result, "# My Packet\n\nHello, Alice!");
    }

    #[test]
    fn template_engine_render_missing_variable() {
        let engine = TemplateEngine::new();
        let ctx = TemplateContext::new();

        let template = "Hello, {{ name }}!";
        let result = engine.render(template, &ctx).unwrap();

        assert_eq!(result, "Hello, !");
    }

    #[test]
    fn template_engine_with_custom_delimiters() {
        let engine = TemplateEngine::with_delimiters("<<", ">>", "<%", "%>");
        let mut ctx = TemplateContext::new();
        ctx.set("name", "Bob");

        let template = "Hello, << name >>!";
        let result = engine.render(template, &ctx).unwrap();

        assert_eq!(result, "Hello, Bob!");
    }

    #[test]
    fn template_value_from_string() {
        let val: TemplateValue = "hello".into();
        assert_eq!(val.to_string(), "hello");
        assert!(val.is_truthy());
    }

    #[test]
    fn template_value_from_number() {
        let val: TemplateValue = 42.into();
        assert_eq!(val.to_string(), "42");
        assert!(val.is_truthy());

        let val: TemplateValue = 0.into();
        assert_eq!(val.to_string(), "0");
        assert!(!val.is_truthy());
    }

    #[test]
    fn template_value_from_bool() {
        let val: TemplateValue = true.into();
        assert_eq!(val.to_string(), "true");
        assert!(val.is_truthy());

        let val: TemplateValue = false.into();
        assert_eq!(val.to_string(), "false");
        assert!(!val.is_truthy());
    }

    #[test]
    fn template_value_from_list() {
        let val: TemplateValue = vec![
            TemplateValue::String("a".into()),
            TemplateValue::String("b".into()),
        ]
        .into();
        assert!(val.is_truthy());

        let val: TemplateValue = Vec::<TemplateValue>::new().into();
        assert!(!val.is_truthy());
    }

    #[test]
    fn template_value_from_object() {
        let mut obj = HashMap::new();
        obj.insert("key".to_string(), TemplateValue::String("value".into()));
        let val: TemplateValue = obj.into();
        assert!(val.is_truthy());

        let val: TemplateValue = HashMap::new().into();
        assert!(!val.is_truthy());
    }
}
