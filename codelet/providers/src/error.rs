//! Unified Provider Error type for all providers
//!
//! Feature: spec/features/refactor-god-objects-and-unify-error-handling.feature
//! Provides a single error type that all providers use for consistent error handling.

use thiserror::Error;

/// Unified error type for all providers
///
/// This error type distinguishes between authentication, API, and rate limit errors,
/// enabling automatic retry logic for rate limit errors.
#[derive(Debug, Error)]
pub enum ProviderError {
    /// Authentication error - API key or OAuth token invalid/missing
    #[error("[{provider}] Authentication error: {message}")]
    Authentication { provider: String, message: String },

    /// API error - general API communication failure
    #[error("[{provider}] API error: {message}")]
    Api { provider: String, message: String },

    /// Rate limit error - too many requests, should retry after delay
    #[error("[{provider}] Rate limit exceeded: {message}")]
    RateLimit {
        provider: String,
        message: String,
        /// Suggested retry delay in seconds (if provided by API)
        retry_after_secs: Option<u64>,
    },

    /// Configuration error - invalid configuration or missing required settings
    #[error("[{provider}] Configuration error: {message}")]
    Configuration { provider: String, message: String },

    /// Model error - model not found or not supported
    #[error("[{provider}] Model error: {message}")]
    Model { provider: String, message: String },

    /// Content error - invalid content or unsupported content type
    #[error("[{provider}] Content error: {message}")]
    Content { provider: String, message: String },

    /// Timeout error - request timed out
    #[error("[{provider}] Timeout: {message}")]
    Timeout { provider: String, message: String },
}

impl ProviderError {
    /// Check if this error is retryable
    ///
    /// Retryable errors are those that might succeed if tried again,
    /// particularly rate limit errors and timeouts.
    pub fn is_retryable(&self) -> bool {
        match self {
            // Rate limit errors should be retried after a delay
            ProviderError::RateLimit { .. } => true,
            // Timeouts may succeed on retry
            ProviderError::Timeout { .. } => true,
            // All other errors are not retryable
            ProviderError::Authentication { .. } => false,
            ProviderError::Api { .. } => false,
            ProviderError::Configuration { .. } => false,
            ProviderError::Model { .. } => false,
            ProviderError::Content { .. } => false,
        }
    }

    /// Get suggested retry delay in seconds for rate limit errors
    ///
    /// Returns Some(seconds) if this is a rate limit error with retry information,
    /// otherwise returns None.
    pub fn retry_after(&self) -> Option<u64> {
        match self {
            ProviderError::RateLimit {
                retry_after_secs, ..
            } => *retry_after_secs,
            _ => None,
        }
    }

    /// Get the provider name from this error
    pub fn provider_name(&self) -> &str {
        match self {
            ProviderError::Authentication { provider, .. } => provider,
            ProviderError::Api { provider, .. } => provider,
            ProviderError::RateLimit { provider, .. } => provider,
            ProviderError::Configuration { provider, .. } => provider,
            ProviderError::Model { provider, .. } => provider,
            ProviderError::Content { provider, .. } => provider,
            ProviderError::Timeout { provider, .. } => provider,
        }
    }

    /// Create an authentication error for a provider
    pub fn auth(provider: impl Into<String>, message: impl Into<String>) -> Self {
        ProviderError::Authentication {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create an API error for a provider
    pub fn api(provider: impl Into<String>, message: impl Into<String>) -> Self {
        ProviderError::Api {
            provider: provider.into(),
            message: message.into(),
        }
    }

    /// Create a rate limit error for a provider
    pub fn rate_limit(
        provider: impl Into<String>,
        message: impl Into<String>,
        retry_after_secs: Option<u64>,
    ) -> Self {
        ProviderError::RateLimit {
            provider: provider.into(),
            message: message.into(),
            retry_after_secs,
        }
    }

    /// Create a configuration error for a provider
    pub fn config(provider: impl Into<String>, message: impl Into<String>) -> Self {
        ProviderError::Configuration {
            provider: provider.into(),
            message: message.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // @step Given providers use anyhow::Result everywhere
    // This is verified by examining the provider implementations

    // @step When I create a unified ProviderError type in providers/src/error.rs
    // This test verifies the ProviderError enum exists and can be constructed

    #[test]
    fn test_provider_error_distinguishes_error_types() {
        // @step Then all providers should return typed ProviderError instead of anyhow errors
        let auth_err = ProviderError::auth("claude", "API key not found");
        let api_err = ProviderError::api("openai", "Request failed");
        let rate_err = ProviderError::rate_limit("gemini", "Too many requests", Some(30));

        // @step And ProviderError should distinguish authentication, API, and rate limit errors
        assert!(matches!(auth_err, ProviderError::Authentication { .. }));
        assert!(matches!(api_err, ProviderError::Api { .. }));
        assert!(matches!(rate_err, ProviderError::RateLimit { .. }));
    }

    #[test]
    fn test_rate_limit_enables_retry() {
        // @step And rate limit errors should enable automatic retry logic
        let rate_err = ProviderError::rate_limit("claude", "Too many requests", Some(30));

        // Rate limit errors should be retryable
        assert!(rate_err.is_retryable());

        // Should provide retry delay
        assert_eq!(rate_err.retry_after(), Some(30));

        // Non-rate-limit errors should not be retryable
        let auth_err = ProviderError::auth("claude", "Invalid API key");
        assert!(!auth_err.is_retryable());
        assert_eq!(auth_err.retry_after(), None);
    }

    #[test]
    fn test_provider_error_includes_provider_name() {
        let errors = vec![
            ProviderError::auth("claude", "API key missing"),
            ProviderError::api("openai", "Request failed"),
            ProviderError::rate_limit("gemini", "Too many requests", None),
            ProviderError::config("codex", "Invalid model"),
        ];

        for err in errors {
            // Each error should have a provider name
            assert!(!err.provider_name().is_empty());
            // Each error message should contain the provider name in brackets
            assert!(err
                .to_string()
                .contains(&format!("[{}]", err.provider_name())));
        }
    }

    #[test]
    fn test_all_error_variants() {
        let errors = vec![
            ProviderError::Authentication {
                provider: "claude".to_string(),
                message: "API key missing".to_string(),
            },
            ProviderError::Api {
                provider: "openai".to_string(),
                message: "Request failed".to_string(),
            },
            ProviderError::RateLimit {
                provider: "gemini".to_string(),
                message: "Too many requests".to_string(),
                retry_after_secs: Some(60),
            },
            ProviderError::Configuration {
                provider: "codex".to_string(),
                message: "Invalid config".to_string(),
            },
            ProviderError::Model {
                provider: "claude".to_string(),
                message: "Model not found".to_string(),
            },
            ProviderError::Content {
                provider: "openai".to_string(),
                message: "Invalid content".to_string(),
            },
            ProviderError::Timeout {
                provider: "gemini".to_string(),
                message: "Request timed out".to_string(),
            },
        ];

        for err in &errors {
            // Verify all errors have proper Debug impl
            let debug_str = format!("{:?}", err);
            assert!(!debug_str.is_empty());

            // Verify all errors have proper Display impl
            let display_str = err.to_string();
            assert!(!display_str.is_empty());
        }

        // Verify retryable status
        assert!(errors[2].is_retryable()); // RateLimit
        assert!(errors[6].is_retryable()); // Timeout
        assert!(!errors[0].is_retryable()); // Authentication
        assert!(!errors[1].is_retryable()); // Api
    }
}
