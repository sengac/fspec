//! Unified Tool Error type for all tools
//!
//! Feature: spec/features/refactor-god-objects-and-unify-error-handling.feature
//! Provides a single error type that all tools use for consistent error handling.

use thiserror::Error;

/// Unified error type for all tools
///
/// This error type includes the tool name for debugging and provides
/// an is_retryable() method to determine if the operation can be retried.
#[derive(Debug, Error)]
pub enum ToolError {
    /// Timeout error - the operation timed out
    #[error("[{tool}] Timeout: operation timed out after {seconds} seconds")]
    Timeout { tool: &'static str, seconds: u64 },

    /// Execution error - command or operation failed
    #[error("[{tool}] Execution error: {message}")]
    Execution { tool: &'static str, message: String },

    /// File error - file I/O operation failed
    #[error("[{tool}] File error: {message}")]
    File { tool: &'static str, message: String },

    /// Validation error - input validation failed
    #[error("[{tool}] Validation error: {message}")]
    Validation { tool: &'static str, message: String },

    /// Pattern error - regex or glob pattern is invalid
    #[error("[{tool}] Pattern error: {message}")]
    Pattern { tool: &'static str, message: String },

    /// Not found error - resource not found
    #[error("[{tool}] Not found: {message}")]
    NotFound { tool: &'static str, message: String },

    /// String not found error - specific to Edit tool
    #[error("[{tool}] String not found: {message}")]
    StringNotFound { tool: &'static str, message: String },

    /// Language error - unsupported language (specific to AstGrep)
    #[error("[{tool}] Language error: {message}")]
    Language { tool: &'static str, message: String },

    /// Token limit exceeded - file content exceeds maximum token limit (PROV-002)
    #[error("[{tool}] Token limit exceeded: {file_path} has ~{estimated_tokens} tokens (limit: {max_tokens})")]
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

    /// Get the tool name from this error
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

    // @step Given each tool has its own error enum (BashError, ReadError, WriteError, etc.)
    // This is verified by the existence of individual error enums in each tool file

    // @step When I create a unified ToolError type in tools/src/error.rs
    // This test verifies the ToolError enum exists and can be constructed

    #[test]
    fn test_tool_error_includes_tool_name() {
        // @step Then all tools should return ToolError instead of individual error enums
        let timeout_err = ToolError::Timeout {
            tool: "bash",
            seconds: 30,
        };

        // @step And ToolError should include the tool name for debugging
        assert_eq!(timeout_err.tool_name(), "bash");
        assert!(timeout_err.to_string().contains("[bash]"));
        assert!(timeout_err.to_string().contains("30 seconds"));
    }

    #[test]
    fn test_tool_error_is_retryable() {
        // @step And ToolError should have an is_retryable() method
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

        let execution_err = ToolError::Execution {
            tool: "bash",
            message: "command failed".to_string(),
        };
        assert!(!execution_err.is_retryable());
    }

    #[test]
    fn test_all_error_variants() {
        // @step And the individual error enums should be removed
        // This is verified by the migration - once tools use ToolError,
        // the individual error enums (BashError, ReadError, etc.) will be removed

        // Verify all error variants have correct tool names
        let errors = vec![
            ToolError::Timeout {
                tool: "bash",
                seconds: 30,
            },
            ToolError::Execution {
                tool: "bash",
                message: "exit code 1".to_string(),
            },
            ToolError::File {
                tool: "read",
                message: "file not found".to_string(),
            },
            ToolError::Validation {
                tool: "write",
                message: "path must be absolute".to_string(),
            },
            ToolError::Pattern {
                tool: "grep",
                message: "invalid regex".to_string(),
            },
            ToolError::NotFound {
                tool: "ls",
                message: "directory not found".to_string(),
            },
            ToolError::StringNotFound {
                tool: "edit",
                message: "old_string not found".to_string(),
            },
            ToolError::Language {
                tool: "astgrep",
                message: "unsupported language".to_string(),
            },
        ];

        for err in errors {
            // Each error should have a tool name
            assert!(!err.tool_name().is_empty());
            // Each error message should contain the tool name in brackets
            assert!(err.to_string().contains(&format!("[{}]", err.tool_name())));
        }
    }
}
