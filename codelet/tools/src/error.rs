//! Unified Tool Error type for all tools
//!
//! Feature: spec/features/refactor-god-objects-and-unify-error-handling.feature
//! Provides a single error type that all tools use for consistent error handling.

use thiserror::Error;

/// Unified error type for all tools
///
/// This error type includes the tool name for debugging and provides
/// an is_retryable() method to determine if the operation can be retried.
///
/// Display messages are user-friendly without redundant prefixes.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Timeout error - the operation timed out
    #[error("Operation timed out after {seconds} seconds")]
    Timeout { tool: &'static str, seconds: u64 },

    /// Execution error - command or operation failed
    #[error("{message}")]
    Execution { tool: &'static str, message: String },

    /// File error - file I/O operation failed
    #[error("{message}")]
    File { tool: &'static str, message: String },

    /// Validation error - input validation failed
    #[error("{message}")]
    Validation { tool: &'static str, message: String },

    /// Pattern error - regex or glob pattern is invalid
    #[error("Invalid pattern: {message}")]
    Pattern { tool: &'static str, message: String },

    /// Not found error - resource not found
    #[error("{message}")]
    NotFound { tool: &'static str, message: String },

    /// String not found error - specific to Edit tool
    #[error("{message}")]
    StringNotFound { tool: &'static str, message: String },

    /// Language error - unsupported language (specific to AstGrep)
    #[error("Unsupported language: {message}")]
    Language { tool: &'static str, message: String },

    /// Token limit exceeded - file content exceeds maximum token limit (PROV-002)
    #[error("Token limit exceeded: {file_path} has ~{estimated_tokens} tokens (limit: {max_tokens})")]
    TokenLimit {
        tool: &'static str,
        file_path: String,
        estimated_tokens: usize,
        max_tokens: usize,
    },
}

impl ToolError {
    /// Check if this error is retryable
    ///
    /// Retryable errors are those that might succeed if tried again,
    /// such as timeouts or temporary file locks.
    pub fn is_retryable(&self) -> bool {
        match self {
            // Timeouts are retryable - the operation might complete on retry
            ToolError::Timeout { .. } => true,
            // All other errors are not retryable by default
            ToolError::Execution { .. } => false,
            ToolError::File { .. } => false,
            ToolError::Validation { .. } => false,
            ToolError::Pattern { .. } => false,
            ToolError::NotFound { .. } => false,
            ToolError::StringNotFound { .. } => false,
            ToolError::Language { .. } => false,
            ToolError::TokenLimit { .. } => false,
        }
    }

    /// Get the tool name from this error (for logging/debugging)
    pub fn tool_name(&self) -> &'static str {
        match self {
            ToolError::Timeout { tool, .. } => tool,
            ToolError::Execution { tool, .. } => tool,
            ToolError::File { tool, .. } => tool,
            ToolError::Validation { tool, .. } => tool,
            ToolError::Pattern { tool, .. } => tool,
            ToolError::NotFound { tool, .. } => tool,
            ToolError::StringNotFound { tool, .. } => tool,
            ToolError::Language { tool, .. } => tool,
            ToolError::TokenLimit { tool, .. } => tool,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_error_display_is_clean() {
        // Execution errors should show just the message
        let err = ToolError::Execution {
            tool: "bash",
            message: "Command failed with exit code 1".to_string(),
        };
        assert_eq!(err.to_string(), "Command failed with exit code 1");
        
        // File errors should show just the message
        let err = ToolError::File {
            tool: "read",
            message: "File not found: /tmp/test.txt".to_string(),
        };
        assert_eq!(err.to_string(), "File not found: /tmp/test.txt");
    }

    #[test]
    fn test_tool_name_still_accessible() {
        let err = ToolError::Execution {
            tool: "bash",
            message: "failed".to_string(),
        };
        assert_eq!(err.tool_name(), "bash");
    }

    #[test]
    fn test_tool_error_is_retryable() {
        let timeout_err = ToolError::Timeout {
            tool: "bash",
            seconds: 30,
        };
        assert!(timeout_err.is_retryable());

        let validation_err = ToolError::Validation {
            tool: "read",
            message: "path must be absolute".to_string(),
        };
        assert!(!validation_err.is_retryable());
    }
}
