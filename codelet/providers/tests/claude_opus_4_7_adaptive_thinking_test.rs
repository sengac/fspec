//! Feature: spec/features/claude-opus-4-7-adaptive-thinking.feature
//! Feature: spec/features/opus-4-7-thinking-mode-appears-inactive-due-to-omitted-display-default.feature
//!
//! This test file validates that Claude Opus 4.7 is correctly recognised as an
//! adaptive-only thinking model.  Tests use PRODUCTION code from codelet_tools
//! and codelet_providers.
//!
//! PROV-079: Opus 4.7 adaptive thinking support.
//! PROV-080: Adaptive thinking config must include display:'summarized'.
//! Note: No per-model constant needed — default-adaptive logic covers all
//! Claude 4.6+ models automatically.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use codelet_tools::facade::{
    is_adaptive_thinking_model, ClaudeThinkingFacade, ThinkingConfigFacade, ThinkingLevel,
    CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6,
};

use codelet_providers::claude::build_beta_headers;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Scenario: Opus 4.7 is detected as adaptive thinking model
    // =========================================================================

    #[test]
    fn test_opus_4_7_is_adaptive_thinking_model() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";

        // @step When I check is_adaptive_thinking_model
        let result = is_adaptive_thinking_model(model);

        // @step Then the result should be true
        assert!(result, "claude-opus-4-7 should be an adaptive thinking model");
    }

    // =========================================================================
    // Scenario: Opus 4.7 returns adaptive thinking config for High level
    // =========================================================================

    #[test]
    fn test_opus_4_7_returns_adaptive_thinking_for_high() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When I request thinking configuration
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.7 should use adaptive thinking"
        );

        // @step And the config should NOT contain "budget_tokens"
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Opus 4.7 should NOT have budget_tokens"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.7 returns adaptive thinking config for Low level
    // =========================================================================

    #[test]
    fn test_opus_4_7_returns_adaptive_thinking_for_low() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is Low
        let level = ThinkingLevel::Low;

        // @step When I request thinking configuration
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for Low level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.7 should use adaptive thinking for Low level"
        );

        // @step And the config should NOT contain "budget_tokens"
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Opus 4.7 should NOT have budget_tokens for Low level"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.7 returns empty config when thinking is Off
    // =========================================================================

    #[test]
    fn test_opus_4_7_returns_empty_config_when_off() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is Off
        let level = ThinkingLevel::Off;

        // @step When I request thinking configuration
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should be empty
        assert!(
            config.is_none(),
            "Off should return None for Opus 4.7 (respects user intent)"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.7 beta headers exclude interleaved-thinking
    // =========================================================================

    #[test]
    fn test_opus_4_7_beta_headers_exclude_interleaved_thinking() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";

        // @step When I build beta headers for the model
        let headers = build_beta_headers(model, false);

        // @step Then the headers should NOT include "interleaved-thinking-2025-05-14"
        assert!(
            !headers.contains("interleaved-thinking-2025-05-14"),
            "Opus 4.7 should NOT include interleaved-thinking, got: {headers}"
        );

        // @step And the headers should include "prompt-caching-2024-07-31"
        assert!(
            headers.contains("prompt-caching-2024-07-31"),
            "Should include prompt-caching, got: {headers}"
        );
    }

    // =========================================================================
    // Scenario: Opus 4.6 adaptive behaviour unchanged after adding 4.7
    // =========================================================================

    #[test]
    fn test_opus_4_6_unchanged_after_adding_4_7() {
        // @step Given the model identifier "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When I request thinking configuration
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should still use adaptive thinking"
        );

        // @step And the config should NOT contain "budget_tokens"
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Opus 4.6 should still NOT have budget_tokens"
        );
    }

    // =========================================================================
    // Scenario: Sonnet 4.6 adaptive behaviour unchanged after adding 4.7
    // =========================================================================

    #[test]
    fn test_sonnet_4_6_unchanged_after_adding_4_7() {
        // @step Given the model identifier "claude-sonnet-4-6"
        let model = CLAUDE_SONNET_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is Medium
        let level = ThinkingLevel::Medium;

        // @step When I request thinking configuration
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for Medium level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Sonnet 4.6 should still use adaptive thinking"
        );

        // @step And the config should NOT contain "budget_tokens"
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Sonnet 4.6 should still NOT have budget_tokens"
        );
    }

    // =========================================================================
    // Scenario: NAPI getThinkingConfig returns adaptive for Opus 4.7
    // (Tested via facade since NAPI calls the same code path)
    // =========================================================================

    #[test]
    fn test_napi_thinking_config_returns_adaptive_for_opus_4_7() {
        // @step Given the NAPI function getThinkingConfig
        // @step And the provider is "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When I call getThinkingConfig
        let config = facade.request_config_for_model(model, level);

        // @step Then the JSON result should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist");
        let config = config.unwrap();
        let json_string = serde_json::to_string(&config).unwrap();
        assert!(
            json_string.contains(r#""type":"adaptive"#),
            "JSON should contain adaptive type, got: {json_string}"
        );

        // @step And the JSON result should NOT contain "budget_tokens"
        assert!(
            !json_string.contains("budget_tokens"),
            "JSON should NOT contain budget_tokens, got: {json_string}"
        );
    }

    // =========================================================================
    // PROV-080: Adaptive thinking config includes display:'summarized'
    // =========================================================================

    // Scenario: Opus 4.7 adaptive config includes display summarized
    #[test]
    fn test_opus_4_7_adaptive_config_includes_display_summarized() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When the thinking config is generated
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.7 should use adaptive thinking"
        );

        // @step And the config should contain thinking display "summarized"
        assert_eq!(
            config["thinking"]["display"].as_str(),
            Some("summarized"),
            "Opus 4.7 adaptive config must include display:'summarized', got: {}",
            serde_json::to_string_pretty(&config).unwrap()
        );
    }

    // Scenario: Opus 4.6 adaptive config also includes display summarized
    #[test]
    fn test_opus_4_6_adaptive_config_includes_display_summarized() {
        // @step Given the model identifier "claude-opus-4-6"
        let model = CLAUDE_OPUS_4_6;
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When the thinking config is generated
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should contain thinking type "adaptive"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking"
        );

        // @step And the config should contain thinking display "summarized"
        assert_eq!(
            config["thinking"]["display"].as_str(),
            Some("summarized"),
            "Opus 4.6 adaptive config must include display:'summarized', got: {}",
            serde_json::to_string_pretty(&config).unwrap()
        );
    }

    // Scenario: Off level returns empty config for adaptive models
    #[test]
    fn test_opus_4_7_off_returns_empty_config() {
        // @step Given the model identifier "claude-opus-4-7"
        let model = "claude-opus-4-7";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is Off
        let level = ThinkingLevel::Off;

        // @step When the thinking config is generated
        let config = facade.request_config_for_model(model, level);

        // @step Then the config should be empty
        assert!(
            config.is_none(),
            "Off level should return None for Opus 4.7"
        );
    }

    // Scenario: Budgeted models remain unchanged with no display field
    #[test]
    fn test_budgeted_models_no_display_field() {
        // @step Given the model identifier "claude-opus-4-5"
        let model = "claude-opus-4-5";
        let facade = ClaudeThinkingFacade;

        // @step And the thinking level is High
        let level = ThinkingLevel::High;

        // @step When the thinking config is generated
        let config = facade.request_config(level);

        // @step Then the config should contain thinking type "enabled"
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Opus 4.5 should use budgeted thinking"
        );

        // @step And the config should contain thinking budget_tokens 32000
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(32000),
            "Opus 4.5 should have budget_tokens 32000"
        );

        // @step And the config should not contain a display field
        assert!(
            config["thinking"]["display"].is_null(),
            "Budgeted models should NOT have a display field, got: {}",
            serde_json::to_string_pretty(&config).unwrap()
        );
        let _ = model; // suppress unused warning
    }
}
