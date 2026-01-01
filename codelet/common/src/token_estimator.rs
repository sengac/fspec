//! Token estimation using tiktoken-rs
//!
//! PROV-002: Provides accurate token counting using tiktoken-rs cl100k_base encoding.
//! Replaces byte-based estimation (len()/4) throughout the codebase.
//!
//! Usage:
//! ```rust
//! use codelet_common::token_estimator::count_tokens;
//!
//! let text = "Hello, world!";
//! let tokens = count_tokens(text);
//! ```

use once_cell::sync::Lazy;
use tiktoken_rs::CoreBPE;

/// Default maximum tokens for file reads (configurable via CODELET_MAX_FILE_TOKENS env var)
pub const DEFAULT_MAX_FILE_TOKENS: usize = 25_000;

/// Global TokenEstimator instance using lazy initialization.
/// Uses cl100k_base encoding which is compatible with GPT-4 and Claude models.
static ESTIMATOR: Lazy<TokenEstimator> = Lazy::new(|| {
    TokenEstimator::new().unwrap_or_else(|e| {
        // Fall back to a dummy estimator that uses byte-based estimation
        // This should never happen in practice, but we need a fallback
        tracing::warn!("Failed to initialize tiktoken encoder: {}. Using byte-based fallback.", e);
        TokenEstimator { encoder: None }
    })
});

/// Token estimator wrapping tiktoken-rs encoder.
///
/// Uses cl100k_base encoding for accurate token counting compatible with
/// modern LLMs (GPT-4, Claude, etc.).
pub struct TokenEstimator {
    encoder: Option<CoreBPE>,
}

impl TokenEstimator {
    /// Create a new TokenEstimator with cl100k_base encoding.
    ///
    /// # Errors
    /// Returns an error if the tiktoken encoder fails to initialize.
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let encoder = tiktoken_rs::cl100k_base()?;
        Ok(Self {
            encoder: Some(encoder),
        })
    }

    /// Count tokens in the given text.
    ///
    /// Returns the number of tokens using cl100k_base encoding.
    /// Falls back to byte-based estimation (len/4) if encoder is unavailable.
    pub fn count_tokens(&self, text: &str) -> usize {
        match &self.encoder {
            Some(encoder) => encoder.encode_with_special_tokens(text).len(),
            None => {
                // Fallback: byte-based estimation (~4 bytes per token)
                text.len().div_ceil(4)
            }
        }
    }
}

/// Count tokens in the given text using the global estimator.
///
/// This is the primary API for token counting throughout the codebase.
/// Uses tiktoken-rs cl100k_base encoding for accurate counting.
///
/// # Example
/// ```rust
/// use codelet_common::token_estimator::count_tokens;
///
/// let tokens = count_tokens("Hello, world!");
/// assert!(tokens > 0);
/// ```
pub fn count_tokens(text: &str) -> usize {
    ESTIMATOR.count_tokens(text)
}

/// Get the maximum file tokens limit from environment or default.
///
/// Reads CODELET_MAX_FILE_TOKENS environment variable, defaults to 25,000.
pub fn max_file_tokens() -> usize {
    std::env::var("CODELET_MAX_FILE_TOKENS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_FILE_TOKENS)
}

/// Check if content exceeds the token limit.
///
/// Returns `Some((estimated_tokens, limit))` if the limit is exceeded,
/// `None` otherwise.
pub fn check_token_limit(content: &str) -> Option<(usize, usize)> {
    let limit = max_file_tokens();
    let tokens = count_tokens(content);
    if tokens > limit {
        Some((tokens, limit))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_basic() {
        // Basic English text should tokenize reasonably
        let text = "Hello, world!";
        let tokens = count_tokens(text);
        assert!(tokens > 0, "Should count at least 1 token");
        assert!(tokens < 10, "Simple phrase should have fewer than 10 tokens");
    }

    #[test]
    fn test_count_tokens_empty() {
        let tokens = count_tokens("");
        assert_eq!(tokens, 0, "Empty string should have 0 tokens");
    }

    #[test]
    fn test_count_tokens_code() {
        let code = "fn main() { println!(\"Hello\"); }";
        let tokens = count_tokens(code);
        assert!(tokens > 0, "Code should have tokens");
    }

    #[test]
    fn test_max_file_tokens_default() {
        // Clear env var to test default
        std::env::remove_var("CODELET_MAX_FILE_TOKENS");
        assert_eq!(max_file_tokens(), DEFAULT_MAX_FILE_TOKENS);
    }

    #[test]
    fn test_max_file_tokens_custom() {
        std::env::set_var("CODELET_MAX_FILE_TOKENS", "50000");
        assert_eq!(max_file_tokens(), 50000);
        std::env::remove_var("CODELET_MAX_FILE_TOKENS");
    }

    #[test]
    fn test_check_token_limit_under() {
        std::env::remove_var("CODELET_MAX_FILE_TOKENS");
        let small_text = "Hello, world!";
        assert!(check_token_limit(small_text).is_none());
    }

    #[test]
    fn test_check_token_limit_over() {
        // Set a very low limit to test
        std::env::set_var("CODELET_MAX_FILE_TOKENS", "1");
        let text = "Hello, world! This is a longer text that will exceed 1 token.";
        let result = check_token_limit(text);
        assert!(result.is_some());
        let (tokens, limit) = result.unwrap();
        assert!(tokens > limit);
        std::env::remove_var("CODELET_MAX_FILE_TOKENS");
    }
}
