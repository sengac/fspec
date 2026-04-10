//! CopilotBehaviorFacade — model-family-specific behaviour (reasoning_effort
//! variants, chat.params mutations, reasoning_opaque round-trip).
//!
//! PROV-055: Rule 6 — this module defines a trait analogous to
//! `ThinkingConfigFacade` with three implementations (GPT / Claude / Gemini),
//! plus a [`select_copilot_behavior_facade`] function that dispatches by
//! model-ID prefix (mirroring `select_claude_facade(is_oauth)` in
//! `codelet/tools/src/facade/system_prompt.rs:427`).
//!
//! The behaviour facade captures things that are **specific to the model
//! family** and therefore cannot live in the shared Copilot header facade:
//!
//! - Which `reasoning_effort` variants the family understands
//! - Any family-specific mutations to the `chat.params` sub-object
//! - How to extract and round-trip a `reasoning_opaque` blob (GPT-5 only, in
//!   practice — Claude and Gemini return `None` / no-op)

use serde_json::Value;
use tracing::warn;

use super::model_family;

/// Behaviour trait for Copilot model families.
pub trait CopilotBehaviorFacade: Send + Sync {
    /// The short family identifier returned by [`select_copilot_behavior_facade`]:
    /// `"gpt"`, `"claude"`, or `"gemini"`.
    fn family(&self) -> &'static str;

    /// The set of `reasoning_effort` variants this family supports. Empty
    /// slice for families that do not expose reasoning effort at all.
    fn reasoning_effort_variants(&self) -> &'static [&'static str];

    /// Mutate the `chat.params` JSON sub-object in-place to apply any
    /// family-specific transforms. Default implementation is a no-op.
    fn mutate_chat_params(&self, _params: &mut Value) {}

    /// Extract a `reasoning_opaque` blob from an API response so it can be
    /// echoed back on the next turn (GPT-5 `/responses` API only).
    ///
    /// Returns `None` for families that do not expose reasoning_opaque.
    fn extract_reasoning_opaque(&self, _response: &Value) -> Option<Value> {
        None
    }

    /// Inject a previously-extracted `reasoning_opaque` blob into the next
    /// turn's request body. Default implementation is a no-op (non-GPT
    /// families do not round-trip reasoning_opaque).
    fn inject_reasoning_opaque(&self, _next_request: &mut Value, _blob: &Value) {}
}

/// Boxed trait object alias — mirrors [`BoxedSystemPromptFacade`] in
/// `codelet/tools/src/facade/system_prompt.rs`.
pub type BoxedCopilotBehaviorFacade = Box<dyn CopilotBehaviorFacade>;

// ============================================================================
// GPT family
// ============================================================================

/// Behaviour facade for GPT-family Copilot models (gpt-4, gpt-4o, gpt-5,
/// gpt-5-codex, gpt-5-mini, gpt-4o-copilot, etc.).
///
/// GPT-5 is the only family that actually round-trips `reasoning_opaque`,
/// but we expose the round-trip on the whole family for forward compatibility.
pub struct CopilotGptBehaviorFacade;

impl CopilotBehaviorFacade for CopilotGptBehaviorFacade {
    fn family(&self) -> &'static str {
        "gpt"
    }

    fn reasoning_effort_variants(&self) -> &'static [&'static str] {
        // GPT-5 models expose low / medium / high reasoning_effort.
        &["low", "medium", "high"]
    }

    fn extract_reasoning_opaque(&self, response: &Value) -> Option<Value> {
        response.get("reasoning_opaque").cloned()
    }

    fn inject_reasoning_opaque(&self, next_request: &mut Value, blob: &Value) {
        if let Some(obj) = next_request.as_object_mut() {
            obj.insert("reasoning_opaque".to_string(), blob.clone());
        }
    }
}

// ============================================================================
// Claude family
// ============================================================================

/// Behaviour facade for Claude-family Copilot models (claude-sonnet-*,
/// claude-opus-*, etc.).
///
/// Claude via Copilot does not expose reasoning_effort or reasoning_opaque —
/// both methods fall back to the trait defaults (empty / no-op).
pub struct CopilotClaudeBehaviorFacade;

impl CopilotBehaviorFacade for CopilotClaudeBehaviorFacade {
    fn family(&self) -> &'static str {
        "claude"
    }

    fn reasoning_effort_variants(&self) -> &'static [&'static str] {
        &[]
    }
}

// ============================================================================
// Gemini family
// ============================================================================

/// Behaviour facade for Gemini-family Copilot models (gemini-2.5-pro,
/// gemini-2.5-flash, etc.).
///
/// Gemini via Copilot does not expose reasoning_effort or reasoning_opaque —
/// both methods fall back to the trait defaults.
pub struct CopilotGeminiBehaviorFacade;

impl CopilotBehaviorFacade for CopilotGeminiBehaviorFacade {
    fn family(&self) -> &'static str {
        "gemini"
    }

    fn reasoning_effort_variants(&self) -> &'static [&'static str] {
        &[]
    }
}

// ============================================================================
// Selector
// ============================================================================

/// Select the correct [`CopilotBehaviorFacade`] for a model ID.
///
/// Dispatch is by **prefix**:
///
/// | Prefix      | Family | Facade                        |
/// |-------------|--------|-------------------------------|
/// | `gpt-*`     | GPT    | [`CopilotGptBehaviorFacade`]    |
/// | `claude-*`  | Claude | [`CopilotClaudeBehaviorFacade`] |
/// | `gemini-*`  | Gemini | [`CopilotGeminiBehaviorFacade`] |
/// | _anything else_ | GPT (default) | [`CopilotGptBehaviorFacade`] |
///
/// Unknown prefixes default to GPT because GPT is the lowest-common-denominator
/// behaviour set and is compatible with most Copilot-exposed models. A
/// `tracing::warn!` is emitted whenever this fallback fires so operators
/// have visibility into new Copilot model families arriving from `/models`
/// before a behaviour facade exists for them (PROV-055 review W5).
#[must_use]
pub fn select_copilot_behavior_facade(model_id: &str) -> BoxedCopilotBehaviorFacade {
    if model_family::is_gpt_model(model_id) {
        Box::new(CopilotGptBehaviorFacade)
    } else if model_family::is_claude_model(model_id) {
        Box::new(CopilotClaudeBehaviorFacade)
    } else if model_family::is_gemini_model(model_id) {
        Box::new(CopilotGeminiBehaviorFacade)
    } else {
        warn!(
            model_id = %model_id,
            "CopilotBehaviorFacade: unknown model family prefix — falling back to GPT behaviour set. Add an explicit facade if this becomes common."
        );
        Box::new(CopilotGptBehaviorFacade)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gpt_family_advertises_reasoning_effort_variants() {
        let f = CopilotGptBehaviorFacade;
        assert_eq!(f.family(), "gpt");
        assert_eq!(f.reasoning_effort_variants(), &["low", "medium", "high"]);
    }

    #[test]
    fn claude_family_has_empty_reasoning_effort_variants() {
        let f = CopilotClaudeBehaviorFacade;
        assert_eq!(f.family(), "claude");
        assert!(f.reasoning_effort_variants().is_empty());
    }

    #[test]
    fn gemini_family_has_empty_reasoning_effort_variants() {
        let f = CopilotGeminiBehaviorFacade;
        assert_eq!(f.family(), "gemini");
        assert!(f.reasoning_effort_variants().is_empty());
    }

    #[test]
    fn gpt_round_trips_reasoning_opaque() {
        let f = CopilotGptBehaviorFacade;
        let response = json!({
            "id": "r1",
            "output": [],
            "reasoning_opaque": "blob_123"
        });
        let extracted = f.extract_reasoning_opaque(&response).unwrap();
        let mut next = json!({ "model": "gpt-5", "input": [] });
        f.inject_reasoning_opaque(&mut next, &extracted);
        assert_eq!(next.get("reasoning_opaque"), Some(&json!("blob_123")));
    }

    #[test]
    fn claude_does_not_extract_reasoning_opaque() {
        let f = CopilotClaudeBehaviorFacade;
        let response = json!({ "id": "r1", "reasoning_opaque": "blob_123" });
        assert!(f.extract_reasoning_opaque(&response).is_none());
    }

    #[test]
    fn selector_dispatches_by_prefix() {
        assert_eq!(select_copilot_behavior_facade("gpt-4").family(), "gpt");
        assert_eq!(
            select_copilot_behavior_facade("gpt-5-codex").family(),
            "gpt"
        );
        assert_eq!(
            select_copilot_behavior_facade("claude-sonnet-4.5").family(),
            "claude"
        );
        assert_eq!(
            select_copilot_behavior_facade("gemini-2.5-pro").family(),
            "gemini"
        );
        // Unknown prefix → gpt
        assert_eq!(select_copilot_behavior_facade("mistral-8b").family(), "gpt");
    }
}
