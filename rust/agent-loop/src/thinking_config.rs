//! Thinking configuration helpers (RPC-072 lift from
//! `rust/napi/src/thinking_config.rs`).
//!
//! The canonical NAPI implementation exposed these functions to
//! TypeScript via `#[napi]`. The agent_loop calls into them directly
//! (`use crate::thinking_config::{get_thinking_config, JsThinkingLevel}`)
//! — so the NAPI-free lift drops the `#[napi]` attributes and replaces
//! `napi::Result<T>` with `Result<T, String>`. All logic is preserved.
//!
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature

use codelet_tools::facade::{
    ClaudeThinkingFacade, Gemini25ThinkingFacade, Gemini3ThinkingFacade, ThinkingConfigFacade,
    ThinkingLevel,
};

/// Thinking intensity level. Pure-Rust mirror of the NAPI-side
/// `JsThinkingLevel` enum — the canonical persistent form on the
/// `BackgroundSession.inner.session_thinking_level` field is
/// `codelet_tools::facade::ThinkingLevel`; this type exists for the
/// `PromptInput`/per-turn-override path that historically went through
/// the JS layer.
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
#[inline]
fn is_codex_provider(provider: &str) -> bool {
    provider == "codex" || provider.ends_with("-codex")
}

/// Check if a provider string represents a Claude model.
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
/// Returns the same JSON string the NAPI-side `get_thinking_config`
/// returns; errors collapse to `String` instead of `napi::Error`.
pub fn get_thinking_config(provider: String, level: JsThinkingLevel) -> Result<String, String> {
    let level: ThinkingLevel = level.into();

    let config = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.request_config(level)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.request_config(level)
    } else if is_claude_provider(&provider) {
        // PROV-005: All Claude models use model-aware config.
        match ClaudeThinkingFacade.request_config_for_model(&provider, level) {
            Some(config) => config,
            None => serde_json::json!({}),
        }
    } else if is_codex_provider(&provider) {
        // PROV-037: Codex/OpenAI reasoning models use OpenAI Responses API format.
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
        serde_json::json!({})
    };

    serde_json::to_string(&config).map_err(|e| format!("Failed to serialize config: {e}"))
}

/// Check if a response part contains thinking content.
pub fn is_thinking_content(provider: String, part_json: String) -> Result<bool, String> {
    let part: serde_json::Value =
        serde_json::from_str(&part_json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let is_thinking = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.is_thinking_part(&part)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.is_thinking_part(&part)
    } else if is_claude_provider(&provider) {
        ClaudeThinkingFacade.is_thinking_part(&part)
    } else {
        false
    };

    Ok(is_thinking)
}

/// Extract thinking text from a response part.
pub fn extract_thinking_text(
    provider: String,
    part_json: String,
) -> Result<Option<String>, String> {
    let part: serde_json::Value =
        serde_json::from_str(&part_json).map_err(|e| format!("Invalid JSON: {e}"))?;

    let text = if is_gemini3_provider(&provider) {
        Gemini3ThinkingFacade.extract_thinking_text(&part)
    } else if is_gemini25_provider(&provider) {
        Gemini25ThinkingFacade.extract_thinking_text(&part)
    } else if is_claude_provider(&provider) {
        ClaudeThinkingFacade.extract_thinking_text(&part)
    } else {
        None
    };

    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_claude_provider_routes_all_claude_models() {
        assert!(is_claude_provider("claude-opus-4-6"));
        assert!(is_claude_provider("claude-sonnet-4-6"));
        assert!(is_claude_provider("claude-opus-4-5"));
        assert!(is_claude_provider("claude-sonnet-4-5"));
        assert!(is_claude_provider("claude-opus-4-6-20260201"));
        assert!(is_claude_provider("claude-sonnet-4-5-20250929"));
        assert!(is_claude_provider("claude"));
        assert!(is_claude_provider("claude-3"));
        assert!(is_claude_provider("claude-opus"));
        assert!(is_claude_provider("claude-sonnet"));
        assert!(is_claude_provider("claude-3-opus"));
        assert!(is_claude_provider("claude-3-sonnet"));
        assert!(is_claude_provider("claude-3.5-sonnet"));
        assert!(is_claude_provider("claude-3.5-haiku"));
        assert!(is_claude_provider("claude-opus-4-7"));
        assert!(is_claude_provider("claude-opus-5-0"));
        assert!(!is_claude_provider("gemini-3"));
        assert!(!is_claude_provider("gpt-4"));
        assert!(!is_claude_provider(""));
    }

    #[test]
    fn test_is_codex_provider_matches_codex_model_patterns() {
        assert!(is_codex_provider("codex"));
        assert!(is_codex_provider("gpt-5.1-codex"));
        assert!(is_codex_provider("gpt-5.3-codex"));
        assert!(is_codex_provider("gpt-6.0-codex"));
        assert!(!is_codex_provider("openai"));
        assert!(!is_codex_provider("claude"));
        assert!(!is_codex_provider("gemini-3"));
        assert!(!is_codex_provider("gpt-4o"));
        assert!(!is_codex_provider(""));
        assert!(!is_codex_provider("my-codex-tool"));
        assert!(!is_codex_provider("codex-helper"));
    }

    #[test]
    fn test_get_thinking_config_codex_returns_reasoning_format() {
        let config = get_thinking_config("codex".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed["reasoning"]["effort"].as_str(), Some("high"));
        assert_eq!(parsed["reasoning"]["summary"].as_str(), Some("auto"));

        let config = get_thinking_config("gpt-5.3-codex".to_string(), JsThinkingLevel::Medium)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed["reasoning"]["effort"].as_str(), Some("medium"));

        let config =
            get_thinking_config("codex".to_string(), JsThinkingLevel::Off).expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(parsed, serde_json::json!({}));
    }

    #[test]
    fn test_gemini_provider_routing() {
        assert!(is_gemini3_provider("gemini-3"));
        assert!(is_gemini3_provider("gemini-3-pro"));
        assert!(is_gemini3_provider("gemini-3-flash"));
        assert!(!is_gemini3_provider("gemini-2.5"));
        assert!(!is_gemini3_provider("claude"));

        assert!(is_gemini25_provider("gemini-2.5"));
        assert!(is_gemini25_provider("gemini-2.5-pro"));
        assert!(is_gemini25_provider("gemini-2.5-flash"));
        assert!(!is_gemini25_provider("gemini-3"));
        assert!(!is_gemini25_provider("claude"));
    }

    #[test]
    fn test_get_thinking_config_routes_claude_to_model_aware() {
        let config = get_thinking_config("claude-opus-4-7".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.7 should use adaptive thinking"
        );

        let config = get_thinking_config("claude-opus-4-6".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("adaptive"),
            "Opus 4.6 should use adaptive thinking"
        );

        let config = get_thinking_config("claude-opus-4-5".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("enabled"),
            "Opus 4.5 should use budgeted thinking"
        );

        let config = get_thinking_config("claude".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed["thinking"]["type"].as_str(),
            Some("enabled"),
            "Generic 'claude' should use budgeted thinking"
        );
    }

    #[test]
    fn test_unknown_provider_returns_empty_config() {
        let config = get_thinking_config("unknown-provider".to_string(), JsThinkingLevel::High)
            .expect("Should succeed");
        let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!({}),
            "Unknown provider should return empty"
        );
    }
}
