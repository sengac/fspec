//! Feature: spec/features/claude-opus-4-6-adaptive-thinking.feature
//!
//! This test file validates the acceptance criteria for Claude Opus 4.6 and Sonnet 4.6
//! adaptive thinking support. Tests use PRODUCTION code from codelet_tools and codelet_providers.
//!
//! PROV-005: Tests verify both the facade logic AND the actual wiring.
#![allow(clippy::expect_used, clippy::unwrap_used)]

// Import from PRODUCTION code - no local redefinitions
use codelet_tools::facade::{
    ClaudeThinkingFacade, ThinkingLevel,
    // Model constants
    CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6, CLAUDE_OPUS_4_5, CLAUDE_SONNET_4_5,
    // Model detection helpers
    is_adaptive_thinking_model, supports_1m_context,
};

// Import beta header builder from claude provider
use codelet_providers::claude::build_beta_headers;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Scenario: Opus 4.6 uses adaptive thinking automatically
    // =========================================================================

    #[test]
    fn test_opus_4_6_uses_adaptive_thinking_automatically() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, ThinkingLevel::Medium);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist for Medium level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Opus 4.6 should NOT have budget_tokens"
        );
    }

    // =========================================================================
    // Scenario: Sonnet 4.6 uses adaptive thinking automatically
    // =========================================================================

    #[test]
    fn test_sonnet_4_6_uses_adaptive_thinking_automatically() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;
        let facade = ClaudeThinkingFacade;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, ThinkingLevel::Medium);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist for Medium level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Sonnet 4.6 should use adaptive thinking"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Sonnet 4.6 should NOT have budget_tokens"
        );
    }

    // =========================================================================
    // Scenario: User-provided budget_tokens is ignored for Opus 4.6
    // =========================================================================

    #[test]
    fn test_budget_tokens_ignored_for_opus_4_6() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set a thinking budget of 16000 tokens
        // Note: The facade ignores budget for adaptive models - we verify the output
        // is adaptive regardless of what level we pass (High has budget 32000 normally)
        let level = ThinkingLevel::High;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking even with budget level"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "budget_tokens should be ignored for Opus 4.6"
        );
    }

    // =========================================================================
    // Scenario: User-provided budget_tokens is ignored for Sonnet 4.6
    // =========================================================================

    #[test]
    fn test_budget_tokens_ignored_for_sonnet_4_6() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set a thinking budget of 16000 tokens
        let level = ThinkingLevel::High;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Sonnet 4.6 should use adaptive thinking even with budget level"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "budget_tokens should be ignored for Sonnet 4.6"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.5 uses budget-based thinking
    // =========================================================================

    #[test]
    fn test_opus_4_5_uses_budget_based_thinking() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-5"
        let model = CLAUDE_OPUS_4_5;
        let facade = ClaudeThinkingFacade;

        // @step And I have set a thinking budget of 16000 tokens
        // Medium level = 16000 tokens in the facade
        let level = ThinkingLevel::Medium;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "enabled"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Opus 4.5 should use budget-based thinking"
        );

        // @step And the request should contain budget_tokens of 16000
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(16000),
            "budget_tokens should be 16000 for Medium level"
        );
    }

    // =========================================================================
    // Scenario: Sonnet 4.5 uses budget-based thinking
    // =========================================================================

    #[test]
    fn test_sonnet_4_5_uses_budget_based_thinking() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-5"
        let model = CLAUDE_SONNET_4_5;
        let facade = ClaudeThinkingFacade;

        // @step And I have set a thinking budget of 16000 tokens
        let level = ThinkingLevel::Medium;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "enabled"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Sonnet 4.5 should use budget-based thinking"
        );

        // @step And the request should contain budget_tokens of 16000
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(16000),
            "budget_tokens should be 16000 for Medium level"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.6 uses correct beta headers
    // =========================================================================

    #[test]
    fn test_opus_4_6_beta_headers() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;

        // @step When I make an API request
        let headers = build_beta_headers(model, false);

        // @step Then the anthropic-beta header should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );

        // @step And the anthropic-beta header should include "context-1m-2025-08-07"
        assert!(
            headers.contains("context-1m-2025-08-07"),
            "Opus 4.6 should include context-1m, got: {headers}"
        );

        // @step And the anthropic-beta header should NOT include "interleaved-thinking-2025-05-14"
        assert!(
            !headers.contains("interleaved-thinking-2025-05-14"),
            "Opus 4.6 should NOT include interleaved-thinking, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Sonnet 4.6 uses correct beta headers
    // =========================================================================

    #[test]
    fn test_sonnet_4_6_beta_headers() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;

        // @step When I make an API request
        let headers = build_beta_headers(model, false);

        // @step Then the anthropic-beta header should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );

        // @step And the anthropic-beta header should include "context-1m-2025-08-07"
        assert!(
            headers.contains("context-1m-2025-08-07"),
            "Sonnet 4.6 should include context-1m, got: {headers}"
        );

        // @step And the anthropic-beta header should NOT include "interleaved-thinking-2025-05-14"
        assert!(
            !headers.contains("interleaved-thinking-2025-05-14"),
            "Sonnet 4.6 should NOT include interleaved-thinking, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.5 uses correct beta headers without 1M context
    // =========================================================================

    #[test]
    fn test_opus_4_5_beta_headers() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-5"
        let model = CLAUDE_OPUS_4_5;

        // @step When I make an API request
        let headers = build_beta_headers(model, false);

        // @step Then the anthropic-beta header should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );

        // @step And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
        assert!(
            headers.contains("interleaved-thinking-2025-05-14"),
            "Opus 4.5 should include interleaved-thinking, got: {headers}"
        );

        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Opus 4.5 should NOT include context-1m, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Sonnet 4.5 uses correct beta headers with 1M context
    // =========================================================================

    #[test]
    fn test_sonnet_4_5_beta_headers() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-5"
        let model = CLAUDE_SONNET_4_5;

        // @step When I make an API request
        let headers = build_beta_headers(model, false);

        // @step Then the anthropic-beta header should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );

        // @step And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
        assert!(
            headers.contains("interleaved-thinking-2025-05-14"),
            "Sonnet 4.5 should include interleaved-thinking, got: {headers}"
        );

        // @step And the anthropic-beta header should include "context-1m-2025-08-07"
        assert!(
            headers.contains("context-1m-2025-08-07"),
            "Sonnet 4.5 should include context-1m, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Unknown model uses default behavior with explicit constant matching
    // =========================================================================

    #[test]
    fn test_unknown_model_uses_default_behavior() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, ThinkingLevel::Medium);

        // @step Then the request should contain thinking configuration with type "enabled"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Unknown model should use budget-based thinking"
        );

        // @step And the request should contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Unknown model should have budget_tokens"
        );

        // Check headers
        let headers = build_beta_headers(model, false);

        // @step And the anthropic-beta header should include "interleaved-thinking-2025-05-14"
        assert!(
            headers.contains("interleaved-thinking-2025-05-14"),
            "Unknown model should include interleaved-thinking, got: {headers}"
        );

        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Unknown model should NOT include context-1m, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Partial model name does not match adaptive thinking models
    // =========================================================================

    #[test]
    fn test_partial_model_name_does_not_match() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6-preview"
        let model = "claude-opus-4-6-preview";
        let facade = ClaudeThinkingFacade;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, ThinkingLevel::Medium);

        // @step Then the request should contain thinking configuration with type "enabled"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Partial match should NOT use adaptive thinking"
        );

        // @step And the request should contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Partial match should have budget_tokens"
        );

        // Check headers
        let headers = build_beta_headers(model, false);

        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Partial match should NOT include context-1m, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Thinking level 'high' defaults to adaptive for Opus 4.6
    // =========================================================================

    #[test]
    fn test_thinking_level_high_defaults_to_adaptive_for_opus_4_6() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set the thinking level to "high"
        let level = ThinkingLevel::High;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "High level should still use adaptive for Opus 4.6"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "High level should NOT have budget_tokens for Opus 4.6"
        );
    }

    // =========================================================================
    // Scenario: Thinking level 'low' defaults to adaptive for Sonnet 4.6
    // =========================================================================

    #[test]
    fn test_thinking_level_low_defaults_to_adaptive_for_sonnet_4_6() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set the thinking level to "low"
        let level = ThinkingLevel::Low;

        // @step When I make an API request with thinking enabled
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should contain thinking configuration with type "adaptive"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Low level should still use adaptive for Sonnet 4.6"
        );

        // @step And the request should NOT contain a budget_tokens field
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Low level should NOT have budget_tokens for Sonnet 4.6"
        );
    }

    // =========================================================================
    // Scenario: Thinking disabled with 'off' for Opus 4.6
    // =========================================================================

    #[test]
    fn test_thinking_disabled_with_off_for_opus_4_6() {
        // @step Given I have configured the Claude provider with model "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set the thinking level to "off"
        let level = ThinkingLevel::Off;

        // @step When I make an API request
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should NOT contain a thinking configuration
        assert!(
            config.is_none(),
            "Off should disable thinking for Opus 4.6"
        );
    }

    // =========================================================================
    // Scenario: Thinking disabled with 'off' for Sonnet 4.6
    // =========================================================================

    #[test]
    fn test_thinking_disabled_with_off_for_sonnet_4_6() {
        // @step Given I have configured the Claude provider with model "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And I have set the thinking level to "off"
        let level = ThinkingLevel::Off;

        // @step When I make an API request
        let config = facade.request_config_for_model(model, level);

        // @step Then the request should NOT contain a thinking configuration
        assert!(
            config.is_none(),
            "Off should disable thinking for Sonnet 4.6"
        );
    }

    // =========================================================================
    // Additional helper function tests - verify PRODUCTION code behavior
    // =========================================================================

    #[test]
    fn test_is_adaptive_thinking_model_uses_exact_equality() {
        // Verify exact equality matching (no .contains() or .starts_with())
        assert!(is_adaptive_thinking_model(CLAUDE_OPUS_4_6));
        assert!(is_adaptive_thinking_model(CLAUDE_SONNET_4_6));
        assert!(!is_adaptive_thinking_model(CLAUDE_OPUS_4_5));
        assert!(!is_adaptive_thinking_model(CLAUDE_SONNET_4_5));
        // These should NOT match - verifies exact equality, not pattern matching
        assert!(!is_adaptive_thinking_model("claude-opus-4-7"));
        assert!(!is_adaptive_thinking_model("claude-opus-4-6-preview"));
        assert!(!is_adaptive_thinking_model("claude-opus-4-6-20260201"));
    }

    #[test]
    fn test_supports_1m_context_uses_exact_equality() {
        // Verify exact equality matching
        assert!(supports_1m_context(CLAUDE_OPUS_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_5));
        assert!(supports_1m_context("claude-sonnet-4-5-20250929")); // Versioned model in list
        // Opus 4.5 does NOT support 1M
        assert!(!supports_1m_context(CLAUDE_OPUS_4_5));
        // Unknown models do not match
        assert!(!supports_1m_context("claude-opus-4-7"));
    }

    #[test]
    fn test_model_constants_have_correct_values() {
        // Verify the constants are what we expect
        assert_eq!(CLAUDE_OPUS_4_6, "claude-opus-4-6");
        assert_eq!(CLAUDE_SONNET_4_6, "claude-sonnet-4-6");
        assert_eq!(CLAUDE_OPUS_4_5, "claude-opus-4-5");
        assert_eq!(CLAUDE_SONNET_4_5, "claude-sonnet-4-5");
    }
}
