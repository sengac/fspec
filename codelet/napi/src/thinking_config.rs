//! NAPI bindings for ThinkingConfigFacade
//!
//! Exposes thinking configuration functionality to TypeScript/Node.js.
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature
//!
//! PROV-005: Updated to support model-aware thinking configuration for Claude.
//! Opus 4.6 and Sonnet 4.6 use adaptive thinking instead of budgeted thinking.
//! All Claude models are routed through the model-aware facade which handles
//! exact equality checks internally (per rule [7]).

use codelet_tools::facade::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
};
use napi_derive::napi;

/// TypeScript-friendly thinking level enum
#[napi]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsThinkingLevel {
    /// Disable thinking/reasoning entirely
    Off,
    /// Minimal thinking (fast responses)
    Low,
    /// Balanced thinking (default for most tasks)
    Medium,
    /// Maximum thinking (complex reasoning tasks)
    High,
}

impl From<JsThinkingLevel> for ThinkingLevel {
    fn from(level: JsThinkingLevel) -> Self {
        match level {
            JsThinkingLevel::Off => ThinkingLevel::Off,
            JsThinkingLevel::Low => ThinkingLevel::Low,
            JsThinkingLevel::Medium => ThinkingLevel::Medium,
            JsThinkingLevel::High => ThinkingLevel::High,
        }
    }
}

/// Check if a provider string represents a Codex or OpenAI reasoning model.
///
/// Matches:
/// - "codex" (generic provider name)
/// - "gpt-*-codex" model names (e.g., "gpt-5.1-codex", "gpt-5.3-codex")
/// - Future codex model variants that end with "-codex"
///
/// NOTE: Does NOT match "openai" because that provider also handles non-reasoning
/// models like gpt-4o. When the session_manager passes a model name like
/// "gpt-5.1-codex" it will match via ends_with("-codex").
#[inline]
fn is_codex_provider(provider: &str) -> bool {
    provider == "codex" || provider.ends_with("-codex")
}

/// Check if a provider string represents a Claude model.
///
/// This is used ONLY for routing to the Claude facade.
/// The facade handles exact equality checks for model-specific behavior (PROV-005 rule [7]).
#[inline]
fn is_claude_provider(provider: &str) -> bool {
    provider.starts_with("claude")
}

/// Check if a provider string represents a Gemini 3 model.
#[inline]
fn is_gemini3_provider(provider: &str) -> bool {
    matches!(
        provider,
        "gemini-3"
            | "gemini-3-pro"
            | "gemini-3-flash"
            | "gemini-3-pro-preview"
            | "gemini-3-flash-preview"
    )
}

/// Check if a provider string represents a Gemini 2.5 model.
#[inline]
fn is_gemini25_provider(provider: &str) -> bool {
    matches!(
        provider,
        "gemini-2.5"
            | "gemini-2.5-pro"
            | "gemini-2.5-flash"
            | "gemini-2.5-pro-preview"
            | "gemini-2.5-flash-preview"
    )
}

/// Get thinking configuration JSON for a provider at a specific level.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", or specific Claude model names
/// * `level` - Thinking intensity level
///
/// # Returns
/// JSON string containing the provider-specific thinking configuration.
/// Returns empty object for Off level or unknown providers.
///
/// # PROV-005: Model-aware Claude thinking
/// All Claude models are routed through the model-aware facade. The facade uses
/// exact equality checks (per rule [7]) to determine whether to use adaptive or
/// budgeted thinking. This ensures no duplicate model detection logic in the NAPI layer.
///
/// # Example
/// ```typescript
/// import { getThinkingConfig, JsThinkingLevel } from '@anthropic/codelet-napi';
///
/// // Gemini - uses thinkingLevel enum
/// const geminiConfig = JSON.parse(getThinkingConfig('gemini-3', JsThinkingLevel.High));
/// // { thinkingConfig: { includeThoughts: true, thinkingLevel: "high" } }
///
/// // Claude 4.6 - uses adaptive thinking (PROV-005)
/// const opus46Config = JSON.parse(getThinkingConfig('claude-opus-4-6', JsThinkingLevel.High));
/// // { thinking: { type: "adaptive" } }
///
/// // Claude 4.5 - uses budgeted thinking
/// const opus45Config = JSON.parse(getThinkingConfig('claude-opus-4-5', JsThinkingLevel.High));
/// // { thinking: { type: "enabled", budget_tokens: 32000 } }
/// ```
#[napi]
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> napi::Result<String> {
    let level: ThinkingLevel = level.into();

    let config = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.request_config(level)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.request_config(level)
    } else if is_claude_provider(&provider) {
        // PROV-005: All Claude models use model-aware config.
        // The facade handles exact equality checks for adaptive vs budgeted thinking.
        // Unknown Claude models fall back to budgeted thinking (safe default).
        match ClaudeThinkingFacade.request_config_for_model(&provider, level) {
            Some(config) => config,
            None => serde_json::json!({}), // Off level returns empty
        }
    } else if is_codex_provider(&provider) {
        // PROV-037: Codex/OpenAI reasoning models use OpenAI Responses API format.
        // The reasoning parameter controls whether the model performs chain-of-thought
        // reasoning before producing output. Without it, GPT-5.x Codex models
        // cannot perform multi-step agentic reasoning and tool use.
        //
        // Reference: codex-rs core/src/client.rs lines 500-553
        // Format matches what codex-rs sends for models with supports_reasoning_summaries=true
        match level {
            ThinkingLevel::Off => serde_json::json!({}),
            ThinkingLevel::Low => serde_json::json!({
                "reasoning": { "effort": "low", "summary": "auto" }
            }),
            ThinkingLevel::Medium => serde_json::json!({
                "reasoning": { "effort": "medium", "summary": "auto" }
            }),
            ThinkingLevel::High => serde_json::json!({
                "reasoning": { "effort": "high", "summary": "auto" }
            }),
        }
    } else {
        // Unknown provider - return empty config (no thinking)
        serde_json::json!({})
    };

    serde_json::to_string(&config).map_err(|e| {
        napi::Error::new(
            napi::Status::GenericFailure,
            format!("Failed to serialize config: {}", e),
        )
    })
}

/// Check if a response part contains thinking content.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", or specific Claude model names
/// * `part_json` - JSON string of the response part
///
/// # Returns
/// true if the part contains thinking/reasoning content, false otherwise.
///
/// # Example
/// ```typescript
/// import { isThinkingContent } from '@anthropic/codelet-napi';
///
/// const part = JSON.stringify({ thought: true, text: "Let me think..." });
/// const isThinking = isThinkingContent('gemini-3', part);
/// // true
/// ```
#[napi]
pub fn is_thinking_content(provider: String, part_json: String) -> napi::Result<bool> {
    let part: serde_json::Value = serde_json::from_str(&part_json)
        .map_err(|e| napi::Error::new(napi::Status::InvalidArg, format!("Invalid JSON: {}", e)))?;

    let is_thinking = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.is_thinking_part(&part)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.is_thinking_part(&part)
    } else if is_claude_provider(&provider) {
        // All Claude models use the same response format for thinking content
        ClaudeThinkingFacade.is_thinking_part(&part)
    } else {
        // Unknown provider - not thinking content
        false
    };

    Ok(is_thinking)
}

/// Extract thinking text from a response part.
///
/// # Arguments
/// * `provider` - Provider identifier: "gemini-3", "gemini-2.5", "claude", or specific Claude model names
/// * `part_json` - JSON string of the response part
///
/// # Returns
/// The thinking text if present, null otherwise.
#[napi]
pub fn extract_thinking_text(provider: String, part_json: String) -> napi::Result<Option<String>> {
    let part: serde_json::Value = serde_json::from_str(&part_json)
        .map_err(|e| napi::Error::new(napi::Status::InvalidArg, format!("Invalid JSON: {}", e)))?;

    let text = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.extract_thinking_text(&part)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.extract_thinking_text(&part)
    } else if is_claude_provider(&provider) {
        // All Claude models use the same response format for thinking content
        ClaudeThinkingFacade.extract_thinking_text(&part)
    } else {
        // Unknown provider - no text
        None
    };

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // PROV-005: Verify routing helpers work correctly
    // =========================================================================

    #[test]
    fn test_is_claude_provider_routes_all_claude_models() {
        // Specific 4.x models
        assert!(is_claude_provider("claude-opus-4-6"));
        assert!(is_claude_provider("claude-sonnet-4-6"));
        assert!(is_claude_provider("claude-opus-4-5"));
        assert!(is_claude_provider("claude-sonnet-4-5"));

        // Versioned models
        assert!(is_claude_provider("claude-opus-4-6-20260201"));
        assert!(is_claude_provider("claude-sonnet-4-5-20250929"));

        // Generic provider names
        assert!(is_claude_provider("claude"));
        assert!(is_claude_provider("claude-3"));
        assert!(is_claude_provider("claude-opus"));
        assert!(is_claude_provider("claude-sonnet"));
        assert!(is_claude_provider("claude-3-opus"));
        assert!(is_claude_provider("claude-3-sonnet"));
        assert!(is_claude_provider("claude-3.5-sonnet"));
        assert!(is_claude_provider("claude-3.5-haiku"));

        // Unknown future models - should still route to Claude
        assert!(is_claude_provider("claude-opus-4-7"));
        assert!(is_claude_provider("claude-opus-5-0"));

        // Non-Claude
        assert!(!is_claude_provider("gemini-3"));
        assert!(!is_claude_provider("gpt-4"));
        assert!(!is_claude_provider(""));
    }

    // =========================================================================
    // PROV-037: Verify codex provider routing helper
    // =========================================================================

    #[test]
    fn test_is_codex_provider_matches_codex_model_patterns() {
        // Generic provider name
        assert!(is_codex_provider("codex"));

        // GPT-x-codex model names
        assert!(is_codex_provider("gpt-5.1-codex"));
        assert!(is_codex_provider("gpt-5.3-codex"));
        assert!(is_codex_provider("gpt-6.0-codex"));

        // Should NOT match "openai" — that provider also handles non-reasoning models
        assert!(!is_codex_provider("openai"));

        // Should NOT match non-codex providers
        assert!(!is_codex_provider("claude"));
        assert!(!is_codex_provider("gemini-3"));
        assert!(!is_codex_provider("gpt-4o"));
        assert!(!is_codex_provider(""));

        // Should NOT match substrings that happen to contain "codex" but don't
        // follow the pattern (exact "codex" or ending with "-codex")
        assert!(!is_codex_provider("my-codex-tool"));
        assert!(!is_codex_provider("codex-helper"));
    }

    // =========================================================================
    // PROV-037: Verify codex thinking config returns correct format
    // =========================================================================

    #[test]
    fn test_get_thinking_config_codex_returns_reasoning_format() {
        // High level
        let config = get_thinking_config("codex".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed["reasoning"]["effort"].as_str(), Some("high"));
        assert_eq!(parsed["reasoning"]["summary"].as_str(), Some("auto"));

        // Medium level
        let config = get_thinking_config("gpt-5.3-codex".to_string(), JsThinkingLevel::Medium)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed["reasoning"]["effort"].as_str(), Some("medium"));

        // Off level — empty config
        let config = get_thinking_config("codex".to_string(), JsThinkingLevel::Off)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed, serde_json::json!({}));
    }

    #[test]
    fn test_gemini_provider_routing() {
        // Gemini 3
        assert!(is_gemini3_provider("gemini-3"));
        assert!(is_gemini3_provider("gemini-3-pro"));
        assert!(is_gemini3_provider("gemini-3-flash"));
        assert!(!is_gemini3_provider("gemini-2.5"));
        assert!(!is_gemini3_provider("claude"));

        // Gemini 2.5
        assert!(is_gemini25_provider("gemini-2.5"));
        assert!(is_gemini25_provider("gemini-2.5-pro"));
        assert!(is_gemini25_provider("gemini-2.5-flash"));
        assert!(!is_gemini25_provider("gemini-3"));
        assert!(!is_gemini25_provider("claude"));
    }

    #[test]
    fn test_get_thinking_config_routes_claude_to_model_aware() {
        // Adaptive thinking model
        let config = get_thinking_config("claude-opus-4-6".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking"
        );

        // Budgeted thinking model
        let config = get_thinking_config("claude-opus-4-5".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("enabled"),
            "Opus 4.5 should use budgeted thinking"
        );

        // Generic "claude" also works (falls back to budgeted)
        let config =
            get_thinking_config("claude".to_string(), JsThinkingLevel::High).expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("enabled"),
            "Generic 'claude' should use budgeted thinking"
        );
    }

    #[test]
    fn test_unknown_provider_returns_empty_config() {
        let config =
            get_thinking_config("unknown-provider".to_string(), JsThinkingLevel::High).expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed, serde_json::json!({}), "Unknown provider should return empty");
    }
}
