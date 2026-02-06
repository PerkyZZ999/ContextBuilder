//! Error types for ContextBuilder.
//!
//! Library crates use [`ContextBuilderError`] via `thiserror`.
//! App crates (cli/tui) wrap this with `color-eyre` for rich diagnostics.

use std::path::PathBuf;

/// Top-level error type for all ContextBuilder operations.
#[derive(Debug, thiserror::Error)]
pub enum ContextBuilderError {
    /// Configuration loading or validation error.
    #[error("config error: {message}")]
    Config { message: String },

    /// Network/HTTP error during crawl or discovery.
    #[error("network error: {0}")]
    Network(String),

    /// HTML parsing or content extraction error.
    #[error("parse error: {message}")]
    Parse { message: String },

    /// Database or storage layer error.
    #[error("storage error: {0}")]
    Storage(String),

    /// LLM enrichment error (bridge, API, or response parsing).
    #[error("enrichment error: {0}")]
    Enrichment(String),

    /// Filesystem I/O error.
    #[error("I/O error at {path:?}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    /// Data validation error (schema mismatch, invalid format, etc.).
    #[error("validation error: {message}")]
    Validation { message: String },

    /// HTML-to-Markdown conversion error.
    #[error("conversion error: {0}")]
    Conversion(String),
}

/// Convenience alias used throughout the codebase.
pub type Result<T> = std::result::Result<T, ContextBuilderError>;

impl ContextBuilderError {
    /// Create a config error from any displayable message.
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config {
            message: msg.into(),
        }
    }

    /// Create a parse error from any displayable message.
    pub fn parse(msg: impl Into<String>) -> Self {
        Self::Parse {
            message: msg.into(),
        }
    }

    /// Create a validation error from any displayable message.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation {
            message: msg.into(),
        }
    }

    /// Wrap a `std::io::Error` with a path for context.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_formatting() {
        let err = ContextBuilderError::config("missing API key");
        assert_eq!(err.to_string(), "config error: missing API key");

        let err = ContextBuilderError::validation("schema_version 99 not supported");
        assert!(err.to_string().contains("schema_version 99"));
    }
}
