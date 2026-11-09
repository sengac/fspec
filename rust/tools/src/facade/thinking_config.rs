//! Thinking/Reasoning Configuration Facades
//!
//! Provider facades that recognise thinking content in responses, and the
//! Claude model helpers (adaptive-thinking detection for the beta headers,
//! 1M context).
//!
//! MODEL-010: request thinking is no longer built here. The agent loop's wire
//! table (thinking-levels-reach-each-provider-through-a-wire-table.feature)
//! builds each provider's form from the session's level; the legacy level
//! tables (Claude and Gemini 2.5 budgets, Gemini 3 / Claude adaptive maps)
//! are gone.
//!
//! Feature: spec/features/thinking-config-facade-for-provider-specific-reasoning.feature

use serde_json::Value;

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

/// Claude Haiku 4.5 model identifier
pub const CLAUDE_HAIKU_4_5: &str = "claude-haiku-4-5";

/// Model prefixes that require budgeted thinking (pre-adaptive era: these
/// reject `{"type": "adaptive"}`). All other Claude 4.x+ models default to
/// adaptive thinking. Uses `starts_with` matching so versioned variants (e.g.
/// "claude-opus-4-5-20260101") are covered. BUG-438: Haiku 4.5 and the
/// Claude 4.0 / 4.1 ids; 4.0 is matched by its dated ids ("claude-opus-4-2025"),
/// because a bare "claude-opus-4" prefix would also match 4.6.
pub const BUDGETED_THINKING_MODELS: &[&str] = &[
    CLAUDE_OPUS_4_5,
    CLAUDE_SONNET_4_5,
    CLAUDE_HAIKU_4_5,
    "claude-opus-4-1",
    "claude-opus-4-2025",
    "claude-sonnet-4-2025",
];

/// Model prefixes that do NOT support 1M context window.
/// All Claude 4.5+ models support 1M context EXCEPT Opus 4.5.
/// Claude 3.x models also do not support 1M context.
pub const NO_1M_CONTEXT_MODELS: &[&str] = &[CLAUDE_OPUS_4_5];

/// Check if a model uses adaptive thinking.
///
/// Default-adaptive: All Claude 4.6+ models use adaptive thinking.
/// Only old models (4.5 and earlier) use budgeted thinking.
/// This is future-proof — new Claude models automatically get adaptive thinking
/// without requiring code changes.
#[inline]
pub fn is_adaptive_thinking_model(model: &str) -> bool {
    // Must be a Claude model (new naming: claude-{family}-{major}-{minor})
    if !model.starts_with("claude-") {
        return false;
    }
    // Claude 3.x uses old naming (claude-3-*) → always budgeted
    if model.starts_with("claude-3") {
        return false;
    }
    // Known budgeted models (prefix match covers versioned variants)
    for budgeted in BUDGETED_THINKING_MODELS {
        if model.starts_with(budgeted) {
            return false;
        }
    }
    // All other Claude models default to adaptive
    true
}

/// Check if a model supports 1M context window.
///
/// Default-enabled: All Claude 4.5+ models support 1M context EXCEPT Opus 4.5.
/// Claude 3.x models do not support 1M context.
#[inline]
pub fn supports_1m_context(model: &str) -> bool {
    // Must be a Claude model
    if !model.starts_with("claude-") {
        return false;
    }
    // Claude 3.x → no 1M support
    if model.starts_with("claude-3") {
        return false;
    }
    // Known models without 1M support (prefix match)
    for no_1m in NO_1M_CONTEXT_MODELS {
        if model.starts_with(no_1m) {
            return false;
        }
    }
    // All other Claude 4.x+ models support 1M
    true
}

/// The CLI session's thinking intensity (PROV-041 exhaustion notices).
///
/// MODEL-010: no request is built from it; the agent loop's wire table
/// (`codelet_agent_loop::thinking_config`) turns a session's level into each
/// provider's form.
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

/// Claude thinking facade: recognises thinking content blocks. MODEL-010:
/// request thinking is built by the agent loop's wire table.
pub struct ClaudeThinkingFacade;

impl ThinkingConfigFacade for ClaudeThinkingFacade {
    fn provider(&self) -> &'static str {
        "claude"
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
    use serde_json::json;

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

    #[test]
    fn test_adaptive_thinking_model_detection() {
        // Default-adaptive: Claude 4.6+ models are adaptive, 4.5 and earlier are budgeted
        assert!(is_adaptive_thinking_model(CLAUDE_OPUS_4_6));
        assert!(is_adaptive_thinking_model(CLAUDE_SONNET_4_6));
        assert!(is_adaptive_thinking_model("claude-opus-4-7")); // Future: adaptive by default
        assert!(is_adaptive_thinking_model("claude-opus-4-8")); // Future: adaptive by default
        assert!(is_adaptive_thinking_model("claude-sonnet-5-0")); // Future: adaptive by default
        assert!(is_adaptive_thinking_model("claude-opus-4-6-preview")); // Variant: adaptive
        assert!(is_adaptive_thinking_model("claude-opus-4-6-20260201")); // Dated: adaptive
        assert!(is_adaptive_thinking_model("claude-haiku-5-0")); // Future family: adaptive
        assert!(!is_adaptive_thinking_model(CLAUDE_OPUS_4_5)); // Old: budgeted
        assert!(!is_adaptive_thinking_model(CLAUDE_SONNET_4_5)); // Old: budgeted
        assert!(!is_adaptive_thinking_model("claude-sonnet-4-5-20250929")); // Old variant: budgeted
        assert!(!is_adaptive_thinking_model("claude-3-opus-20240229")); // Claude 3.x: budgeted
        assert!(!is_adaptive_thinking_model("claude-3-5-sonnet-20241022")); // Claude 3.x: budgeted
        assert!(!is_adaptive_thinking_model("gemini-3")); // Non-Claude: not adaptive
        assert!(!is_adaptive_thinking_model("claude")); // Generic (no dash suffix): not adaptive
    }

    #[test]
    fn test_context_1m_support_detection() {
        // Default-enabled for Claude 4.5+ (except Opus 4.5)
        assert!(supports_1m_context(CLAUDE_OPUS_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_6));
        assert!(supports_1m_context(CLAUDE_SONNET_4_5));
        assert!(supports_1m_context("claude-sonnet-4-5-20250929")); // Variant auto-covered
        assert!(supports_1m_context("claude-opus-4-7")); // Future: auto-covered
        assert!(supports_1m_context("claude-opus-4-8")); // Future: auto-covered
        assert!(!supports_1m_context(CLAUDE_OPUS_4_5)); // Explicit exception
        assert!(!supports_1m_context("claude-opus-4-5-20260101")); // Variant of exception
        assert!(!supports_1m_context("claude-3-opus-20240229")); // Claude 3.x: no 1M
        assert!(!supports_1m_context("gemini-3")); // Non-Claude
    }
}
