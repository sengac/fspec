//! System Prompt Facades for Provider-Specific Prompt Handling
//!
//! This module implements the facade pattern for system prompts, allowing different
//! LLM providers to receive system prompts in their expected format.
//!
//! # Provider Differences
//!
//! - **Claude OAuth**: Requires "You are Claude Code..." prefix, uses array format with cache_control
//! - **Claude API Key**: No prefix, but uses array format with cache_control
//! - **Gemini/OpenAI**: Use plain string format, no special transformation

use serde_json::{json, Value};

/// Claude Code system prompt prefix (required for OAuth authentication)
pub const CLAUDE_CODE_PROMPT_PREFIX: &str =
    "You are Claude Code, Anthropic's official CLI for Claude.";

/// Trait for provider-specific system prompt formatting.
///
/// Each facade adapts system prompt formatting for a specific LLM provider,
/// handling differences in prefix requirements, array vs string format,
/// and cache_control metadata.
pub trait SystemPromptFacade: Send + Sync {
    /// Returns the provider this facade is for (e.g., "claude", "gemini", "openai")
    fn provider(&self) -> &'static str;

    /// Returns the identity prefix if required for this provider/auth mode.
    ///
    /// For Claude OAuth, returns Some("You are Claude Code...").
    /// For other providers/modes, returns None.
    fn identity_prefix(&self) -> Option<&'static str>;

    /// Transform the preamble according to provider requirements.
    ///
    /// This may prepend an identity prefix (for Claude OAuth) or
    /// pass through unchanged (for other providers).
    fn transform_preamble(&self, preamble: &str) -> String;

    /// Format the system prompt for the provider's API.
    ///
    /// Returns the properly formatted Value for the provider:
    /// - Claude: JSON array with cache_control blocks
    /// - Gemini/OpenAI: Plain string
    fn format_for_api(&self, preamble: &str) -> Value;
}

// ============================================================================
// Claude OAuth System Prompt Facade
// ============================================================================

/// Claude OAuth system prompt facade.
///
/// Formats system prompts for Claude with OAuth authentication:
/// - Prepends "You are Claude Code..." identity prefix
/// - Uses array format with cache_control for prompt caching
pub struct ClaudeOAuthSystemPromptFacade;

impl SystemPromptFacade for ClaudeOAuthSystemPromptFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        Some(CLAUDE_CODE_PROMPT_PREFIX)
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        format!("{CLAUDE_CODE_PROMPT_PREFIX}\n\n{preamble}")
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // OAuth mode: Handle empty preamble case (PROV-006)
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let trimmed = preamble.trim();
        if trimmed.is_empty() {
            // Only prefix - put cache_control on the prefix itself
            json!([
                {
                    "type": "text",
                    "text": CLAUDE_CODE_PROMPT_PREFIX,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        } else {
            // 2 blocks:
            // 1. Claude Code prefix WITHOUT cache_control (static, always same)
            // 2. Preamble WITH cache_control (variable content to cache)
            json!([
                {
                    "type": "text",
                    "text": CLAUDE_CODE_PROMPT_PREFIX
                },
                {
                    "type": "text",
                    "text": preamble,
                    "cache_control": { "type": "ephemeral" }
                }
            ])
        }
    }
}

// ============================================================================
// Claude API Key System Prompt Facade
// ============================================================================

/// Claude API Key system prompt facade.
///
/// Formats system prompts for Claude with API key authentication:
/// - No identity prefix (passes preamble through unchanged)
/// - Uses array format with cache_control for prompt caching
pub struct ClaudeApiKeySystemPromptFacade;

impl SystemPromptFacade for ClaudeApiKeySystemPromptFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        preamble.to_string()
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // API key mode: single block with cache_control
        json!([
            {
                "type": "text",
                "text": preamble,
                "cache_control": { "type": "ephemeral" }
            }
        ])
    }
}

// ============================================================================
// Gemini System Prompt Facade
// ============================================================================

/// Gemini system prompt facade.
///
/// Formats system prompts for Gemini:
/// - No identity prefix
/// - Plain string format (no special transformation)
pub struct GeminiSystemPromptFacade;

impl SystemPromptFacade for GeminiSystemPromptFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        preamble.to_string()
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // Gemini uses plain string format
        Value::String(preamble.to_string())
    }
}

// ============================================================================
// OpenAI System Prompt Facade
// ============================================================================

/// OpenAI system prompt facade.
///
/// Formats system prompts for OpenAI:
/// - No identity prefix
/// - Plain string format (no special transformation)
pub struct OpenAISystemPromptFacade;

impl SystemPromptFacade for OpenAISystemPromptFacade {
    fn provider(&self) -> &'static str {
        "openai"
    }

    fn identity_prefix(&self) -> Option<&'static str> {
        None
    }

    fn transform_preamble(&self, preamble: &str) -> String {
        preamble.to_string()
    }

    fn format_for_api(&self, preamble: &str) -> Value {
        // OpenAI uses plain string format
        Value::String(preamble.to_string())
    }
}

// ============================================================================
// Facade Selector for ClaudeProvider Integration
// ============================================================================

/// Boxed system prompt facade for dynamic dispatch
pub type BoxedSystemPromptFacade = Box<dyn SystemPromptFacade>;

/// Select the appropriate Claude system prompt facade based on OAuth status
///
/// This function is the integration point for ClaudeProvider (Rule 6: TOOL-008)
/// to select the correct facade based on authentication mode.
///
/// # Arguments
/// * `is_oauth` - True if using OAuth authentication (token starts with "cc-")
///
/// # Returns
/// The appropriate facade implementation
pub fn select_claude_facade(is_oauth: bool) -> BoxedSystemPromptFacade {
    if is_oauth {
        Box::new(ClaudeOAuthSystemPromptFacade)
    } else {
        Box::new(ClaudeApiKeySystemPromptFacade)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_oauth_facade_has_identity_prefix() {
        let facade = ClaudeOAuthSystemPromptFacade;
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_some());
        assert!(facade
            .identity_prefix()
            .unwrap()
            .starts_with("You are Claude Code"));
    }

    #[test]
    fn test_claude_api_key_facade_no_identity_prefix() {
        let facade = ClaudeApiKeySystemPromptFacade;
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_gemini_facade_no_identity_prefix() {
        let facade = GeminiSystemPromptFacade;
        assert_eq!(facade.provider(), "gemini");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_openai_facade_no_identity_prefix() {
        let facade = OpenAISystemPromptFacade;
        assert_eq!(facade.provider(), "openai");
        assert!(facade.identity_prefix().is_none());
    }

    #[test]
    fn test_claude_oauth_transform_preamble() {
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.transform_preamble("Hello");
        assert!(result.contains("You are Claude Code"));
        assert!(result.contains("Hello"));
    }

    #[test]
    fn test_claude_api_key_transform_preamble() {
        let facade = ClaudeApiKeySystemPromptFacade;
        let result = facade.transform_preamble("Hello");
        assert_eq!(result, "Hello");
    }

    #[test]
    fn test_gemini_format_for_api_returns_string() {
        let facade = GeminiSystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_string());
        assert_eq!(result.as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_openai_format_for_api_returns_string() {
        let facade = OpenAISystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_string());
        assert_eq!(result.as_str().unwrap(), "Hello");
    }

    #[test]
    fn test_claude_oauth_format_for_api_returns_array() {
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        // First block is prefix without cache_control
        assert!(arr[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert!(arr[0].get("cache_control").is_none());
        // Second block is preamble with cache_control
        assert_eq!(arr[1]["text"].as_str().unwrap(), "Hello");
        assert!(arr[1].get("cache_control").is_some());
    }

    #[test]
    fn test_claude_api_key_format_for_api_returns_array_with_cache_control() {
        let facade = ClaudeApiKeySystemPromptFacade;
        let result = facade.format_for_api("Hello");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["text"].as_str().unwrap(), "Hello");
        assert!(arr[0].get("cache_control").is_some());
        assert_eq!(
            arr[0]["cache_control"]["type"].as_str().unwrap(),
            "ephemeral"
        );
    }

    #[test]
    fn test_claude_oauth_format_for_api_empty_preamble() {
        // PROV-006: Empty preamble should result in single block with cache_control on prefix
        // Anthropic API rejects: "cache_control cannot be set for empty text blocks"
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1, "Empty preamble should produce single block");
        assert!(arr[0]["text"]
            .as_str()
            .unwrap()
            .starts_with("You are Claude Code"));
        assert!(
            arr[0].get("cache_control").is_some(),
            "Single block should have cache_control"
        );
        assert_eq!(arr[0]["cache_control"]["type"].as_str().unwrap(), "ephemeral");
    }

    #[test]
    fn test_claude_oauth_format_for_api_whitespace_preamble() {
        // Whitespace-only preamble should also result in single block
        let facade = ClaudeOAuthSystemPromptFacade;
        let result = facade.format_for_api("   ");
        assert!(result.is_array());
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 1, "Whitespace preamble should produce single block");
    }

    #[test]
    fn test_select_claude_facade_oauth() {
        let facade = select_claude_facade(true);
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_some());
        assert!(facade
            .identity_prefix()
            .unwrap()
            .starts_with("You are Claude Code"));
    }

    #[test]
    fn test_select_claude_facade_api_key() {
        let facade = select_claude_facade(false);
        assert_eq!(facade.provider(), "claude");
        assert!(facade.identity_prefix().is_none());
    }
}
