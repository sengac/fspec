//! Feature: spec/features/claude-opus-4-6-adaptive-thinking.feature
//!
//! Integration test for NAPI thinking config wiring.
//! Verifies that get_thinking_config() correctly handles model names
//! to trigger adaptive thinking for 4.6 models.
//!
//! PROV-005: This tests the critical wiring that was missing - the NAPI
//! function must receive the MODEL NAME (e.g., "claude-opus-4-6"), not just
//! the provider name ("claude"), to trigger adaptive thinking.
//!
//! NOTE: These tests require the real NAPI bindings (not noop stubs),
//! so they are gated behind `not(feature = "noop")`.

#[cfg(all(test, not(feature = "noop")))]
mod tests {
    use codelet_napi::{get_thinking_config, JsThinkingLevel};

    // =========================================================================
    // NAPI Wiring Tests - Verify model names trigger adaptive thinking
    // =========================================================================

    /// Test that passing exact model "claude-opus-4-6" to NAPI triggers adaptive thinking
    #[test]
    fn test_napi_opus_4_6_returns_adaptive_thinking() {
        // @step Given I call get_thinking_config with model "claude-opus-4-6"
        let result = get_thinking_config("claude-opus-4-6".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the thinking type should be "adaptive"
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "NAPI should return adaptive thinking for claude-opus-4-6, got: {config_str}"
        );

        // @step And budget_tokens should NOT be present
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Should NOT have budget_tokens for adaptive thinking, got: {config_str}"
        );
    }

    /// Test that passing exact model "claude-sonnet-4-6" to NAPI triggers adaptive thinking
    #[test]
    fn test_napi_sonnet_4_6_returns_adaptive_thinking() {
        // @step Given I call get_thinking_config with model "claude-sonnet-4-6"
        let result = get_thinking_config("claude-sonnet-4-6".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the thinking type should be "adaptive"
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "NAPI should return adaptive thinking for claude-sonnet-4-6, got: {config_str}"
        );

        // @step And budget_tokens should NOT be present
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Should NOT have budget_tokens for adaptive thinking, got: {config_str}"
        );
    }

    /// Test that passing "claude-opus-4-5" to NAPI returns budgeted thinking
    #[test]
    fn test_napi_opus_4_5_returns_budgeted_thinking() {
        // @step Given I call get_thinking_config with model "claude-opus-4-5"
        let result = get_thinking_config("claude-opus-4-5".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the thinking type should be "enabled" (budgeted)
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "NAPI should return budgeted thinking for claude-opus-4-5, got: {config_str}"
        );

        // @step And budget_tokens should be present
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Should have budget_tokens for budgeted thinking, got: {config_str}"
        );
    }

    /// Test that passing generic "claude" falls back to budgeted thinking
    /// This is the CURRENT bug behavior - when only provider name is passed
    #[test]
    fn test_napi_generic_claude_returns_budgeted_thinking() {
        // @step Given I call get_thinking_config with generic provider "claude"
        let result = get_thinking_config("claude".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the thinking type should be "enabled" (budgeted - not adaptive!)
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Generic 'claude' should return budgeted thinking (not model-aware), got: {config_str}"
        );

        // @step And budget_tokens should be present
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Generic 'claude' should have budget_tokens, got: {config_str}"
        );
    }

    /// Test that Off level returns empty config even for adaptive models
    #[test]
    fn test_napi_opus_4_6_off_returns_empty() {
        // @step Given I call get_thinking_config with model "claude-opus-4-6" and Off level
        let result = get_thinking_config("claude-opus-4-6".to_string(), JsThinkingLevel::Off);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the result should be empty object
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config,
            serde_json::json!({}),
            "Off level should return empty object, got: {config_str}"
        );
    }

    /// Test partial model names don't match adaptive thinking
    #[test]
    fn test_napi_partial_model_name_uses_budgeted() {
        // @step Given I call get_thinking_config with partial model "claude-opus-4-6-preview"
        let result = get_thinking_config("claude-opus-4-6-preview".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // @step And the thinking type should be "enabled" (budgeted) - not adaptive
        // The NAPI uses starts_with() to ROUTE to model-aware handler, but the actual
        // adaptive thinking decision uses exact equality via is_adaptive_thinking_model().
        // So "claude-opus-4-6-preview" routes to model-aware, then fails exact match, falls to budgeted.
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Partial model name should NOT trigger adaptive thinking, got: {config_str}"
        );
    }

    /// Test versioned model names that aren't explicitly listed
    #[test]
    fn test_napi_versioned_model_uses_budgeted() {
        // @step Given I call get_thinking_config with versioned model "claude-opus-4-6-20260201"
        let result = get_thinking_config("claude-opus-4-6-20260201".to_string(), JsThinkingLevel::High);

        // @step Then the result should be valid JSON
        assert!(result.is_ok(), "Should return valid result");
        let config_str = result.unwrap();

        // Per the feature file, versioned models must be EXPLICITLY added to capability lists.
        // "claude-opus-4-6-20260201" is not in ADAPTIVE_THINKING_MODELS, so it uses budgeted.
        // The NAPI routes via starts_with(), but is_adaptive_thinking_model() uses exact equality.
        let config: serde_json::Value = serde_json::from_str(&config_str)
            .expect("Should be valid JSON");
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Versioned model not in list should use budgeted thinking, got: {config_str}"
        );
    }
}
