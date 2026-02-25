//! Thinking/Reasoning Configuration Facades
//!
//! This module implements provider-specific thinking/reasoning configuration
//! using the facade pattern. Each provider receives thinking config in their
//! native format while the codebase uses a common ThinkingLevel abstraction.
//!
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature
//!
//! PROV-005: Added adaptive thinking support for Claude Opus 4.6 and Sonnet 4.6.
//! These models use `{"type": "adaptive"}` instead of budgeted thinking.

use serde_json::{json, Value};

// =============================================================================
// CLAUDE MODEL CONSTANTS (PROV-005)
// =============================================================================

/// Claude Opus 4.6 model identifier
pub const CLAUDE_OPUS_4_6: &str = "claude-opus-4-6";

/// Claude Sonnet 4.6 model identifier
pub const CLAUDE_SONNET_4_6: &str = "claude-sonnet-4-6";

/// Claude Sonnet 4.5 model identifier
pub const CLAUDE_SONNET_4_5: &str = "claude-sonnet-4-5";

/// Claude Opus 4.5 model identifier
pub const CLAUDE_OPUS_4_5: &str = "claude-opus-4-5";

/// Models that use adaptive thinking (exact equality checks - PROV-005)
/// Per official Anthropic docs, these models use `{"type": "adaptive"}` instead of budgeted thinking.
pub const ADAPTIVE_THINKING_MODELS: &[&str] = &[CLAUDE_OPUS_4_6, CLAUDE_SONNET_4_6];

/// Models that support 1M context window (exact equality checks - PROV-005)
/// Note: Opus 4.5 does NOT support 1M context.
pub const CONTEXT_1M_MODELS: &[&str] = &[
    CLAUDE_OPUS_4_6,
    CLAUDE_SONNET_4_6,
    CLAUDE_SONNET_4_5,
    "claude-sonnet-4-5-20250929", // Versioned model
];

/// Check if a model uses adaptive thinking.
/// Uses exact equality, NOT pattern matching (PROV-005 rule [7]).
#[inline]
pub fn is_adaptive_thinking_model(model: &str) -> bool {
    ADAPTIVE_THINKING_MODELS.contains(&model)
}

/// Check if a model supports 1M context window.
/// Uses exact equality, NOT pattern matching (PROV-005 rule [7]).
#[inline]
pub fn supports_1m_context(model: &str) -> bool {
    CONTEXT_1M_MODELS.contains(&model)
}

/// Provider-agnostic thinking intensity levels.
///
/// Maps to provider-specific configurations:
/// - Gemini 3: thinkingLevel enum ("low", "medium", "high")
/// - Gemini 2.5: thinkingBudget token count
/// - Claude: thinking.budget_tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingLevel {
    /// Disable thinking/reasoning entirely
    Off,
    /// Minimal thinking (fast responses)
    Low,
    /// Balanced thinking (default for most tasks)
    Medium,
    /// Maximum thinking (complex reasoning tasks)
    High,
}

/// Trait for provider-specific thinking configuration facades.
pub trait ThinkingConfigFacade {
    /// Returns the provider identifier (e.g., "gemini-3", "claude")
    fn provider(&self) -> &'static str;

    /// Generates the request configuration JSON for the specified thinking level
    fn request_config(&self, level: ThinkingLevel) -> Value;

    /// Checks if a response part contains thinking content
    fn is_thinking_part(&self, part: &Value) -> bool;

    /// Extracts thinking text from a response part (if it's a thinking part)
    fn extract_thinking_text(&self, part: &Value) -> Option<String>;
}

/// Gemini 3 thinking facade - uses thinkingLevel enum
pub struct Gemini3ThinkingFacade;

impl ThinkingConfigFacade for Gemini3ThinkingFacade {
    fn provider(&self) -> &'static str {
        "gemini-3"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "low"
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "medium"
                }
            }),
            ThinkingLevel::High => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingLevel": "high"
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        part.get("thought")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("text").and_then(|v| v.as_str()).map(String::from)
        } else {
            None
        }
    }
}

/// Gemini 2.5 thinking facade - uses thinkingBudget token count
pub struct Gemini25ThinkingFacade;

impl ThinkingConfigFacade for Gemini25ThinkingFacade {
    fn provider(&self) -> &'static str {
        "gemini-2.5"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 2048
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 4096
                }
            }),
            ThinkingLevel::High => json!({
                "thinkingConfig": {
                    "includeThoughts": true,
                    "thinkingBudget": 8192
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        // Gemini 2.5 uses same response format as Gemini 3
        part.get("thought")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false)
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("text").and_then(|v| v.as_str()).map(String::from)
        } else {
            None
        }
    }
}

/// Claude thinking facade - uses thinking.type + budget_tokens (budgeted models)
/// or thinking.type: "adaptive" (4.6 models)
///
/// PROV-005: This facade now supports both budgeted and adaptive thinking:
/// - Opus 4.6, Sonnet 4.6: Always returns `{"type": "adaptive"}` (budget ignored)
/// - Opus 4.5, Sonnet 4.5, older: Returns `{"type": "enabled", "budget_tokens": N}`
pub struct ClaudeThinkingFacade;

impl ClaudeThinkingFacade {
    /// Build thinking configuration for a specific Claude model.
    ///
    /// PROV-005: Model-aware thinking configuration.
    /// - For Opus/Sonnet 4.6: Returns adaptive thinking, ignoring user budget
    /// - For other models: Returns budgeted thinking based on level
    /// - For ThinkingLevel::Off: Always returns None (respects user intent)
    ///
    /// # Arguments
    /// * `model` - The Claude model name (e.g., "claude-opus-4-6")
    /// * `level` - The thinking intensity level
    ///
    /// # Returns
    /// Some(Value) with thinking config, or None for Off level
    pub fn request_config_for_model(&self, model: &str, level: ThinkingLevel) -> Option<Value> {
        // Off always disables thinking (respects user intent - PROV-005 rule [15])
        if level == ThinkingLevel::Off {
            return None;
        }

        // PROV-005: Adaptive thinking models (Opus 4.6, Sonnet 4.6)
        // User-provided budget_tokens and thinking levels (low/med/high) are ignored
        // - they all default to adaptive mode
        if is_adaptive_thinking_model(model) {
            return Some(json!({
                "thinking": {
                    "type": "adaptive"
                }
            }));
        }

        // Budget-based thinking for other models
        Some(self.request_config(level))
    }
}

impl ThinkingConfigFacade for ClaudeThinkingFacade {
    fn provider(&self) -> &'static str {
        "claude"
    }

    fn request_config(&self, level: ThinkingLevel) -> Value {
        match level {
            ThinkingLevel::Off => json!({}),
            ThinkingLevel::Low => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 4096
                }
            }),
            ThinkingLevel::Medium => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 16000
                }
            }),
            ThinkingLevel::High => json!({
                "thinking": {
                    "type": "enabled",
                    "budget_tokens": 32000
                }
            }),
        }
    }

    fn is_thinking_part(&self, part: &Value) -> bool {
        // Claude uses content blocks with type "thinking"
        part.get("type").and_then(|v| v.as_str()) == Some("thinking")
    }

    fn extract_thinking_text(&self, part: &Value) -> Option<String> {
        if self.is_thinking_part(part) {
            part.get("thinking")
                .and_then(|v| v.as_str())
                .map(String::from)
        } else {
            None
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    // =========================================================================
    // Scenario: Gemini 3 facade generates thinkingLevel configuration for High level
    // =========================================================================

    #[test]
    fn test_gemini3_high_level_returns_thinking_level_enum() {
        // @step Given a Gemini3ThinkingFacade
        let facade = Gemini3ThinkingFacade;

        // @step And ThinkingLevel::High is requested
        let level = ThinkingLevel::High;

        // @step When I call request_config with the level
        let config = facade.request_config(level);

        // @step Then the result should contain thinkingConfig.thinkingLevel set to "high"
        assert_eq!(
            config["thinkingConfig"]["thinkingLevel"].as_str(),
            Some("high"),
            "Gemini 3 should use thinkingLevel 'high' for High level"
        );

        // @step And the result should contain thinkingConfig.includeThoughts set to true
        assert_eq!(
            config["thinkingConfig"]["includeThoughts"].as_bool(),
            Some(true),
            "includeThoughts should be true"
        );

        // @step And the result should NOT contain thinkingBudget
        assert!(
            config["thinkingConfig"]["thinkingBudget"].is_null(),
            "Gemini 3 should NOT use thinkingBudget"
        );
    }

    // =========================================================================
    // Scenario: Gemini 2.5 facade generates thinkingBudget configuration for High level
    // =========================================================================

    #[test]
    fn test_gemini25_high_level_returns_thinking_budget() {
        // @step Given a Gemini25ThinkingFacade
        let facade = Gemini25ThinkingFacade;

        // @step And ThinkingLevel::High is requested
        let level = ThinkingLevel::High;

        // @step When I call request_config with the level
        let config = facade.request_config(level);

        // @step Then the result should contain thinkingConfig.thinkingBudget set to 8192
        assert_eq!(
            config["thinkingConfig"]["thinkingBudget"].as_u64(),
            Some(8192),
            "Gemini 2.5 should use thinkingBudget 8192 for High level"
        );

        // @step And the result should contain thinkingConfig.includeThoughts set to true
        assert_eq!(
            config["thinkingConfig"]["includeThoughts"].as_bool(),
            Some(true),
            "includeThoughts should be true"
        );

        // @step And the result should NOT contain thinkingLevel
        assert!(
            config["thinkingConfig"]["thinkingLevel"].is_null(),
            "Gemini 2.5 should NOT use thinkingLevel enum"
        );
    }

    // =========================================================================
    // Scenario: Claude facade generates thinking configuration with budget_tokens
    // =========================================================================

    #[test]
    fn test_claude_high_level_returns_budget_tokens() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And ThinkingLevel::High is requested
        let level = ThinkingLevel::High;

        // @step When I call request_config with the level
        let config = facade.request_config(level);

        // @step Then the result should contain thinking.type set to "enabled"
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Claude should use thinking.type 'enabled'"
        );

        // @step And the result should contain thinking.budget_tokens set to 32000
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(32000),
            "Claude should use budget_tokens 32000 for High level"
        );
    }

    // =========================================================================
    // Scenario: Gemini facade identifies thinking parts in response
    // =========================================================================

    #[test]
    fn test_gemini3_identifies_thinking_parts() {
        // @step Given a Gemini3ThinkingFacade
        let facade = Gemini3ThinkingFacade;

        // @step And a response part with "thought" field set to true
        let thinking_part = json!({
            "thought": true,
            "text": "Let me think about this..."
        });

        // @step When I call is_thinking_part with the part
        let is_thinking = facade.is_thinking_part(&thinking_part);

        // @step Then it should return true
        assert!(
            is_thinking,
            "Part with thought:true should be identified as thinking"
        );

        // @step And extract_thinking_text should return the text content
        let text = facade.extract_thinking_text(&thinking_part);
        assert_eq!(
            text,
            Some("Let me think about this...".to_string()),
            "Should extract thinking text"
        );
    }

    // =========================================================================
    // Scenario: Gemini facade ignores non-thinking parts
    // =========================================================================

    #[test]
    fn test_gemini3_ignores_non_thinking_parts() {
        // @step Given a Gemini3ThinkingFacade
        let facade = Gemini3ThinkingFacade;

        // @step And a response part without "thought" field
        let response_part = json!({
            "text": "The answer is 42."
        });

        // @step When I call is_thinking_part with the part
        let is_thinking = facade.is_thinking_part(&response_part);

        // @step Then it should return false
        assert!(
            !is_thinking,
            "Part without thought field should not be thinking"
        );

        // @step And extract_thinking_text should return None
        let text = facade.extract_thinking_text(&response_part);
        assert!(text.is_none(), "Should return None for non-thinking part");
    }

    // =========================================================================
    // Scenario: ThinkingLevel Off returns empty configuration for all providers
    // =========================================================================

    #[test]
    fn test_thinking_level_off_returns_empty_for_gemini3() {
        // @step Given any ThinkingConfigFacade implementation
        let facade = Gemini3ThinkingFacade;

        // @step And ThinkingLevel::Off is requested
        let level = ThinkingLevel::Off;

        // @step When I call request_config with the level
        let config = facade.request_config(level);

        // @step Then the result should be an empty object
        assert_eq!(config, json!({}), "Off should return empty object");
    }

    #[test]
    fn test_thinking_level_off_returns_empty_for_gemini25() {
        let facade = Gemini25ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Off);
        assert_eq!(config, json!({}), "Off should return empty object");
    }

    #[test]
    fn test_thinking_level_off_returns_empty_for_claude() {
        let facade = ClaudeThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Off);
        assert_eq!(config, json!({}), "Off should return empty object");
    }

    // =========================================================================
    // Additional tests for Low and Medium levels
    // =========================================================================

    #[test]
    fn test_gemini3_low_level() {
        let facade = Gemini3ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Low);

        assert_eq!(
            config["thinkingConfig"]["thinkingLevel"].as_str(),
            Some("low"),
            "Gemini 3 Low should use thinkingLevel 'low'"
        );
        assert_eq!(
            config["thinkingConfig"]["includeThoughts"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_gemini3_medium_level() {
        let facade = Gemini3ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Medium);

        assert_eq!(
            config["thinkingConfig"]["thinkingLevel"].as_str(),
            Some("medium"),
            "Gemini 3 Medium should use thinkingLevel 'medium'"
        );
        assert_eq!(
            config["thinkingConfig"]["includeThoughts"].as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_gemini25_low_level() {
        let facade = Gemini25ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Low);

        assert_eq!(
            config["thinkingConfig"]["thinkingBudget"].as_u64(),
            Some(2048),
            "Gemini 2.5 Low should use thinkingBudget 2048"
        );
    }

    #[test]
    fn test_gemini25_medium_level() {
        let facade = Gemini25ThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Medium);

        assert_eq!(
            config["thinkingConfig"]["thinkingBudget"].as_u64(),
            Some(4096),
            "Gemini 2.5 Medium should use thinkingBudget 4096"
        );
    }

    #[test]
    fn test_claude_low_level() {
        let facade = ClaudeThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Low);

        assert_eq!(config["thinking"]["type"].as_str(), Some("enabled"));
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(4096),
            "Claude Low should use budget_tokens 4096"
        );
    }

    #[test]
    fn test_claude_medium_level() {
        let facade = ClaudeThinkingFacade;
        let config = facade.request_config(ThinkingLevel::Medium);

        assert_eq!(config["thinking"]["type"].as_str(), Some("enabled"));
        assert_eq!(
            config["thinking"]["budget_tokens"].as_u64(),
            Some(16000),
            "Claude Medium should use budget_tokens 16000"
        );
    }

    // =========================================================================
    // Provider identification tests
    // =========================================================================

    #[test]
    fn test_gemini3_provider_name() {
        let facade = Gemini3ThinkingFacade;
        assert_eq!(facade.provider(), "gemini-3");
    }

    #[test]
    fn test_gemini25_provider_name() {
        let facade = Gemini25ThinkingFacade;
        assert_eq!(facade.provider(), "gemini-2.5");
    }

    #[test]
    fn test_claude_provider_name() {
        let facade = ClaudeThinkingFacade;
        assert_eq!(facade.provider(), "claude");
    }

    // =========================================================================
    // Scenario: TypeScript can get thinking configuration via NAPI bindings
    // Note: NAPI bindings tested via this Rust test simulating the flow
    // =========================================================================

    #[test]
    fn test_napi_get_thinking_config() {
        // @step Given the NAPI getThinkingConfig function
        // The NAPI function will call Gemini3ThinkingFacade.request_config()
        let facade = Gemini3ThinkingFacade;

        // @step And provider "gemini-3" and ThinkingLevel.High
        let provider = "gemini-3";
        let level = ThinkingLevel::High;

        // @step When I call getThinkingConfig from TypeScript
        // Simulated: TypeScript would call the NAPI binding which calls this
        let config = facade.request_config(level);
        let json_string = serde_json::to_string(&config).unwrap();

        // @step Then I should receive a JSON string with the correct configuration
        assert!(!json_string.is_empty());
        assert!(json_string.contains("thinkingConfig") || json_string == "{}");

        // @step And the parsed JSON should match the Rust facade output
        let parsed: serde_json::Value = serde_json::from_str(&json_string).unwrap();
        assert_eq!(parsed, config);
        assert_eq!(facade.provider(), provider);
    }

    // =========================================================================
    // Scenario: TypeScript can check if content is thinking via NAPI bindings
    // =========================================================================

    #[test]
    fn test_napi_is_thinking_content() {
        // @step Given the NAPI isThinkingContent function
        let facade = Gemini3ThinkingFacade;

        // @step And a Gemini response part JSON string with thought:true
        let thinking_part = json!({
            "thought": true,
            "text": "Analyzing..."
        });
        let part_json = serde_json::to_string(&thinking_part).unwrap();

        // @step When I call isThinkingContent from TypeScript
        // Simulated: TypeScript would call the NAPI binding which calls this
        let parsed: serde_json::Value = serde_json::from_str(&part_json).unwrap();
        let is_thinking = facade.is_thinking_part(&parsed);

        // @step Then it should return true
        assert!(
            is_thinking,
            "NAPI should correctly identify thinking content"
        );
    }

    // =========================================================================
    // PROV-005: Model-aware Claude thinking configuration tests
    // =========================================================================

    #[test]
    fn test_claude_opus_4_6_returns_adaptive_thinking() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And model "claude-opus-4-6" with ThinkingLevel::High
        let model = CLAUDE_OPUS_4_6;
        let level = ThinkingLevel::High;

        // @step When I call request_config_for_model
        let config = facade.request_config_for_model(model, level);

        // @step Then the result should contain thinking.type set to "adaptive"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking"
        );

        // @step And the result should NOT contain budget_tokens
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Opus 4.6 should NOT have budget_tokens"
        );
    }

    #[test]
    fn test_claude_sonnet_4_6_returns_adaptive_thinking() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And model "claude-sonnet-4-6" with ThinkingLevel::Medium
        let model = CLAUDE_SONNET_4_6;
        let level = ThinkingLevel::Medium;

        // @step When I call request_config_for_model
        let config = facade.request_config_for_model(model, level);

        // @step Then the result should contain thinking.type set to "adaptive"
        assert!(config.is_some(), "Config should exist for Medium level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Sonnet 4.6 should use adaptive thinking"
        );

        // @step And the result should NOT contain budget_tokens
        assert!(
            config["thinking"]["budget_tokens"].is_null(),
            "Sonnet 4.6 should NOT have budget_tokens"
        );
    }

    #[test]
    fn test_claude_opus_4_5_returns_budgeted_thinking() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And model "claude-opus-4-5" with ThinkingLevel::High
        let model = CLAUDE_OPUS_4_5;
        let level = ThinkingLevel::High;

        // @step When I call request_config_for_model
        let config = facade.request_config_for_model(model, level);

        // @step Then the result should contain thinking.type set to "enabled"
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Opus 4.5 should use budgeted thinking"
        );

        // @step And the result should contain budget_tokens
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Opus 4.5 should have budget_tokens"
        );
    }

    #[test]
    fn test_claude_thinking_off_returns_none_for_adaptive_model() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And model "claude-opus-4-6" with ThinkingLevel::Off
        let model = CLAUDE_OPUS_4_6;
        let level = ThinkingLevel::Off;

        // @step When I call request_config_for_model
        let config = facade.request_config_for_model(model, level);

        // @step Then the result should be None
        assert!(
            config.is_none(),
            "Off should return None even for adaptive thinking models"
        );
    }

    #[test]
    fn test_claude_unknown_model_uses_budgeted_thinking() {
        // @step Given a ClaudeThinkingFacade
        let facade = ClaudeThinkingFacade;

        // @step And unknown model "claude-opus-4-7" with ThinkingLevel::High
        let model = "claude-opus-4-7";
        let level = ThinkingLevel::High;

        // @step When I call request_config_for_model
        let config = facade.request_config_for_model(model, level);

        // @step Then the result should use budgeted thinking (default behavior)
        assert!(config.is_some(), "Config should exist for High level");
        let config = config.unwrap();
        assert_eq!(
            config["thinking"]["type"].as_str(),
            Some("enabled"),
            "Unknown model should use budgeted thinking"
        );
        assert!(
            config["thinking"]["budget_tokens"].as_u64().is_some(),
            "Unknown model should have budget_tokens"
        );
    }

    #[test]
    fn test_adaptive_thinking_model_detection() {
        // PROV-005: Verify exact equality matching
        assert!(is_adaptive_thinking_model(CLAUDE_OPUS_4_6));
        assert!(is_adaptive_thinking_model(CLAUDE_SONNET_4_6));
        assert!(!is_adaptive_thinking_model(CLAUDE_OPUS_4_5));
        assert!(!is_adaptive_thinking_model(CLAUDE_SONNET_4_5));
        assert!(!is_adaptive_thinking_model("claude-opus-4-6-preview")); // Partial match
        assert!(!is_adaptive_thinking_model("claude-opus-4-7")); // Unknown
    }

    #[test]
    fn test_context_1m_support_detection() {
        // PROV-005: Verify exact equality matching for 1M context
        assert!(supports_1m_context(CLAUDE_OPUS_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_5));
        assert!(supports_1m_context("claude-sonnet-4-5-20250929")); // Versioned
        assert!(!supports_1m_context(CLAUDE_OPUS_4_5)); // Opus 4.5 does NOT support 1M
        assert!(!supports_1m_context("claude-opus-4-7")); // Unknown
    }
}
