//! Shared Copilot model-family detection utilities (PROV-058 / PROV-055).
//!
//! Centralises the `model_id → family` prefix checks that are used by
//! multiple sibling modules ([`behavior_facade`](super::behavior_facade),
//! [`prompt_cache`](super::prompt_cache)).  Having a single source of truth
//! means a new model prefix only needs to be added in one place.

/// Returns `true` if the model ID identifies a Claude-family model on
/// the Copilot proxy (prefix `"claude-"`).
///
/// Used by:
/// - [`super::prompt_cache::inject_cache_control`] — to gate
///   `copilot_cache_control` injection.
/// - [`super::behavior_facade::select_copilot_behavior_facade`] — to
///   dispatch the Claude behaviour set.
#[must_use]
#[inline]
pub fn is_claude_model(model_id: &str) -> bool {
    model_id.starts_with("claude-")
}

/// Returns `true` if the model ID identifies a GPT-family model on the
/// Copilot proxy (prefix `"gpt-"`).
#[must_use]
#[inline]
pub fn is_gpt_model(model_id: &str) -> bool {
    model_id.starts_with("gpt-")
}

/// Returns `true` if the model ID identifies a Gemini-family model on
/// the Copilot proxy (prefix `"gemini-"`).
#[must_use]
#[inline]
pub fn is_gemini_model(model_id: &str) -> bool {
    model_id.starts_with("gemini-")
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn claude_family_detection() {
        assert!(is_claude_model("claude-sonnet-4"));
        assert!(is_claude_model("claude-sonnet-4.5"));
        assert!(is_claude_model("claude-opus-4.5"));
        assert!(is_claude_model("claude-haiku-4.5"));
        assert!(!is_claude_model("gpt-5"));
        assert!(!is_claude_model("gemini-2.5-pro"));
        assert!(!is_claude_model("mistral-8b"));
    }

    #[test]
    fn gpt_family_detection() {
        assert!(is_gpt_model("gpt-5"));
        assert!(is_gpt_model("gpt-4o"));
        assert!(is_gpt_model("gpt-5-codex"));
        assert!(!is_gpt_model("claude-sonnet-4"));
        assert!(!is_gpt_model("gemini-2.5-pro"));
    }

    #[test]
    fn gemini_family_detection() {
        assert!(is_gemini_model("gemini-2.5-pro"));
        assert!(is_gemini_model("gemini-2.5-flash"));
        assert!(!is_gemini_model("gpt-5"));
        assert!(!is_gemini_model("claude-sonnet-4"));
    }
}
