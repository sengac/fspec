//! Feature: spec/features/claude-opus-4-6-adaptive-thinking.feature
//!
//! This test file validates the acceptance criteria for Claude Opus 4.6 and Sonnet 4.6
//! adaptive thinking support. Tests use PRODUCTION code from codelet_tools and codelet_providers.
//!
//! PROV-005: Tests verify both the facade logic AND the actual wiring.
#![allow(clippy::expect_used, clippy::unwrap_used)]

// Import from PRODUCTION code - no local redefinitions
use codelet_tools::facade::{
    // Model detection helpers
    is_adaptive_thinking_model,
    supports_1m_context,
    CLAUDE_OPUS_4_5,
    // Model constants
    CLAUDE_OPUS_4_6,
    CLAUDE_SONNET_4_5,
    CLAUDE_SONNET_4_6,
};

// Import beta header builder from claude provider
use codelet_providers::claude::build_beta_headers;

#[cfg(test)]
mod tests {
    use super::*;

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

        // CONFIG-007: context-1m header is NOT sent until user opt-in is implemented.
        // Sending it by default causes "Extra usage required" for non-Tier-4 users.
        // (until CONFIG-007)
        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Opus 4.6 should NOT include context-1m until CONFIG-007, got: {headers}"
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

        // CONFIG-007: context-1m header is NOT sent until user opt-in is implemented.
        // (until CONFIG-007)
        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Sonnet 4.6 should NOT include context-1m until CONFIG-007, got: {headers}"
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

        // CONFIG-007: context-1m header is NOT sent until user opt-in is implemented.
        // (until CONFIG-007)
        // @step And the anthropic-beta header should NOT include "context-1m-2025-08-07"
        assert!(
            !headers.contains("context-1m-2025-08-07"),
            "Sonnet 4.5 should NOT include context-1m until CONFIG-007, got: {headers}"
        );
    }

    // =========================================================================
    // Additional helper function tests - verify PRODUCTION code behavior
    // =========================================================================

    #[test]
    fn test_is_adaptive_thinking_model_uses_default_adaptive_logic() {
        // Default-adaptive: Claude 4.6+ are adaptive, 4.5 and earlier are budgeted
        assert!(is_adaptive_thinking_model(CLAUDE_OPUS_4_6));
        assert!(is_adaptive_thinking_model(CLAUDE_SONNET_4_6));
        assert!(!is_adaptive_thinking_model(CLAUDE_OPUS_4_5));
        assert!(!is_adaptive_thinking_model(CLAUDE_SONNET_4_5));
        // Future models default to adaptive (no constant needed)
        assert!(is_adaptive_thinking_model("claude-opus-4-8"));
        // Variants inherit behavior from base model prefix
        assert!(is_adaptive_thinking_model("claude-opus-4-6-preview"));
        assert!(is_adaptive_thinking_model("claude-opus-4-6-20260201"));
        // Old variant is still budgeted
        assert!(!is_adaptive_thinking_model("claude-sonnet-4-5-20250929"));
    }

    #[test]
    fn test_supports_1m_context_uses_default_enabled_logic() {
        // Default-enabled for Claude 4.5+ (except Opus 4.5)
        assert!(supports_1m_context(CLAUDE_OPUS_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_5));
        assert!(supports_1m_context("claude-sonnet-4-5-20250929")); // Variant auto-covered
        assert!(supports_1m_context("claude-opus-4-7")); // Future: auto-covered
                                                         // Opus 4.5 does NOT support 1M
        assert!(!supports_1m_context(CLAUDE_OPUS_4_5));
        // Claude 3.x does not support 1M
        assert!(!supports_1m_context("claude-3-opus-20240229"));
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
