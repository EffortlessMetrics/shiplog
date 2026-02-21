//! Error handling utilities for shiplog.
//!
//! This crate provides error types and utilities for consistent error handling
//! across the shiplog ecosystem.

use std::fmt;

/// Error category for shiplog errors
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Parse,
    Validation,
    Io,
    Config,
    Network,
    Authentication,
    RateLimit,
    Timeout,
    Unknown,
}

impl fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCategory::Parse => write!(f, "parse"),
            ErrorCategory::Validation => write!(f, "validation"),
            ErrorCategory::Io => write!(f, "io"),
            ErrorCategory::Config => write!(f, "config"),
            ErrorCategory::Network => write!(f, "network"),
            ErrorCategory::Authentication => write!(f, "authentication"),
            ErrorCategory::RateLimit => write!(f, "rate_limit"),
            ErrorCategory::Timeout => write!(f, "timeout"),
            ErrorCategory::Unknown => write!(f, "unknown"),
        }
    }
}

/// Shiplog error with category and context
#[derive(Debug)]
pub struct ShiplogError {
    message: String,
    category: ErrorCategory,
    source: Option<Box<dyn std::error::Error>>,
    context: Vec<(String, String)>,
}

impl ShiplogError {
    pub fn new(message: impl Into<String>, category: ErrorCategory) -> Self {
        Self {
            message: message.into(),
            category,
            source: None,
            context: Vec::new(),
        }
    }

    pub fn with_source(
        message: impl Into<String>,
        category: ErrorCategory,
        source: Box<dyn std::error::Error>,
    ) -> Self {
        Self {
            message: message.into(),
            category,
            source: Some(source),
            context: Vec::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn category(&self) -> &ErrorCategory {
        &self.category
    }

    pub fn context(&self) -> &[(String, String)] {
        &self.context
    }

    pub fn is_parse_error(&self) -> bool {
        self.category == ErrorCategory::Parse
    }

    pub fn is_validation_error(&self) -> bool {
        self.category == ErrorCategory::Validation
    }

    pub fn is_io_error(&self) -> bool {
        self.category == ErrorCategory::Io
    }
}

impl fmt::Display for ShiplogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.category, self.message)?;
        
        if !self.context.is_empty() {
            write!(f, " (")?;
            for (i, (key, value)) in self.context.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}={}", key, value)?;
            }
            write!(f, ")")?;
        }
        
        if let Some(source) = &self.source {
            write!(f, "\nCaused by: {}", source)?;
        }
        
        Ok(())
    }
}

impl std::error::Error for ShiplogError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e.as_ref() as _)
    }
}

/// Result type alias for shiplog errors
pub type Result<T> = std::result::Result<T, ShiplogError>;

/// Error builder for constructing errors with context
pub struct ErrorBuilder {
    message: String,
    category: ErrorCategory,
    source: Option<Box<dyn std::error::Error>>,
    context: Vec<(String, String)>,
}

impl ErrorBuilder {
    pub fn new(message: impl Into<String>, category: ErrorCategory) -> Self {
        Self {
            message: message.into(),
            category,
            source: None,
            context: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: Box<dyn std::error::Error>) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> ShiplogError {
        ShiplogError {
            message: self.message,
            category: self.category,
            source: self.source,
            context: self.context,
        }
    }
}

/// Convenience function to create parse errors
pub fn parse_error(message: impl Into<String>) -> ShiplogError {
    ShiplogError::new(message, ErrorCategory::Parse)
}

/// Convenience function to create validation errors
pub fn validation_error(message: impl Into<String>) -> ShiplogError {
    ShiplogError::new(message, ErrorCategory::Validation)
}

/// Convenience function to create IO errors
pub fn io_error(message: impl Into<String>) -> ShiplogError {
    ShiplogError::new(message, ErrorCategory::Io)
}

/// Convenience function to create config errors
pub fn config_error(message: impl Into<String>) -> ShiplogError {
    ShiplogError::new(message, ErrorCategory::Config)
}

/// Convenience function to create network errors
pub fn network_error(message: impl Into<String>) -> ShiplogError {
    ShiplogError::new(message, ErrorCategory::Network)
}

/// Convert anyhow errors to shiplog errors
impl From<anyhow::Error> for ShiplogError {
    fn from(err: anyhow::Error) -> Self {
        ShiplogError::with_source(
            err.to_string(),
            ErrorCategory::Unknown,
            err.into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_error_category_display() {
        assert_eq!(format!("{}", ErrorCategory::Parse), "parse");
        assert_eq!(format!("{}", ErrorCategory::Validation), "validation");
        assert_eq!(format!("{}", ErrorCategory::Io), "io");
    }

    #[test]
    fn test_shiplog_error_creation() {
        let err = ShiplogError::new("test error", ErrorCategory::Parse);
        assert_eq!(err.message(), "test error");
        assert!(matches!(err.category(), ErrorCategory::Parse));
    }

    #[test]
    fn test_shiplog_error_with_context() {
        let err = ShiplogError::new("validation failed", ErrorCategory::Validation)
            .with_context("field", "email")
            .with_context("value", "invalid");
        
        let ctx = err.context();
        assert_eq!(ctx.len(), 2);
        assert_eq!(ctx[0], ("field".to_string(), "email".to_string()));
    }

    #[test]
    fn test_error_builder() {
        let err = ErrorBuilder::new("test error", ErrorCategory::Network)
            .with_context("host", "example.com")
            .with_context("port", "8080")
            .build();
        
        assert_eq!(err.category(), &ErrorCategory::Network);
        assert_eq!(err.context().len(), 2);
    }

    #[test]
    fn test_convenience_functions() {
        let parse = parse_error("invalid format");
        assert!(parse.is_parse_error());

        let validation = validation_error("missing field");
        assert!(validation.is_validation_error());

        let io = io_error("file not found");
        assert!(io.is_io_error());

        let config = config_error("invalid config");
        assert_eq!(config.category(), &ErrorCategory::Config);

        let network = network_error("connection refused");
        assert_eq!(network.category(), &ErrorCategory::Network);
    }

    #[test]
    fn test_error_display() {
        let err = ShiplogError::new("test error", ErrorCategory::Parse);
        let display = format!("{}", err);
        assert!(display.contains("[parse]"));
        assert!(display.contains("test error"));
    }

    #[test]
    fn test_error_display_with_context() {
        let err = ShiplogError::new("test error", ErrorCategory::Validation)
            .with_context("field", "name");
        
        let display = format!("{}", err);
        assert!(display.contains("field=name"));
    }

    #[test]
    fn test_anyhow_conversion() {
        let anyhow_err = anyhow::anyhow!("original error");
        let shiplog_err: ShiplogError = anyhow_err.into();
        
        assert_eq!(shiplog_err.category(), &ErrorCategory::Unknown);
        assert!(shiplog_err.source().is_some());
    }
}
