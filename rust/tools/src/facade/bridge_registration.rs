//! BridgeTool registration utilities
//!
//! Provides helper functions to create BridgeToolFacadeWrapper instances
//! for use in agent builders and tool collections.
//!
//! Feature: spec/features/bridge-tool-unit.feature
//!
//! ## TOOL-012: Session ID at Construction
//!
//! All registration functions accept a `session_id` parameter. This ensures
//! the tool wrapper knows which session's context to use at call time,
//! eliminating reliance on global current session state.

use super::bridge_facade::{
    ClaudeBridgeFacade, GeminiBridgeFacade, OpenAIBridgeFacade, ZAIBridgeFacade,
};
use super::wrapper::BridgeToolFacadeWrapper;
use std::sync::Arc;
use uuid::Uuid;

/// Create a BridgeTool wrapper for Claude provider with explicit session association (TOOL-012)
///
/// # Arguments
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn claude_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(ClaudeBridgeFacade), session_id)
}

/// Create a BridgeTool wrapper for Gemini provider with explicit session association (TOOL-012)
///
/// # Arguments
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn gemini_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(GeminiBridgeFacade), session_id)
}

/// Create a BridgeTool wrapper for OpenAI provider with explicit session association (TOOL-012)
///
/// # Arguments
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn openai_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(OpenAIBridgeFacade), session_id)
}

/// Create a BridgeTool wrapper for Z.AI provider with explicit session association (TOOL-012)
///
/// # Arguments
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn zai_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(ZAIBridgeFacade), session_id)
}

/// Create a BridgeTool wrapper for Codex provider with explicit session association (TOOL-015)
///
/// Codex reuses OpenAIBridgeFacade since both use OpenAI-compatible function calling format.
///
/// # Arguments
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn codex_bridge_tool(session_id: Uuid) -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(OpenAIBridgeFacade), session_id)
}

/// Create a BridgeTool wrapper for the specified provider with explicit session association (TOOL-012)
///
/// # Arguments
/// * `provider` - Provider name ("claude", "gemini", "openai", "zai", "codex")
/// * `session_id` - The session ID for context lookup (must be registered via set_bridge_session_context)
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn bridge_tool_for_provider(
    provider: &str,
    session_id: Uuid,
) -> Option<BridgeToolFacadeWrapper> {
    match provider {
        "claude" => Some(claude_bridge_tool(session_id)),
        "gemini" => Some(gemini_bridge_tool(session_id)),
        "openai" => Some(openai_bridge_tool(session_id)),
        "zai" => Some(zai_bridge_tool(session_id)),
        "codex" => Some(codex_bridge_tool(session_id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_provider_tools_created() {
        let session_id = Uuid::new_v4();

        let claude = claude_bridge_tool(session_id);
        assert_eq!(claude.provider(), "claude");
        assert_eq!(claude.session_id(), session_id);

        let gemini = gemini_bridge_tool(session_id);
        assert_eq!(gemini.provider(), "gemini");
        assert_eq!(gemini.session_id(), session_id);

        let openai = openai_bridge_tool(session_id);
        assert_eq!(openai.provider(), "openai");
        assert_eq!(openai.session_id(), session_id);

        let zai = zai_bridge_tool(session_id);
        assert_eq!(zai.provider(), "zai");
        assert_eq!(zai.session_id(), session_id);

        let codex = codex_bridge_tool(session_id);
        assert_eq!(codex.provider(), "openai"); // Reuses OpenAIBridgeFacade
        assert_eq!(codex.session_id(), session_id);
    }

    #[test]
    fn test_provider_lookup() {
        let session_id = Uuid::new_v4();

        assert!(bridge_tool_for_provider("claude", session_id).is_some());
        assert!(bridge_tool_for_provider("gemini", session_id).is_some());
        assert!(bridge_tool_for_provider("openai", session_id).is_some());
        assert!(bridge_tool_for_provider("zai", session_id).is_some());
        assert!(bridge_tool_for_provider("codex", session_id).is_some());
        assert!(bridge_tool_for_provider("unknown", session_id).is_none());
    }

    #[test]
    fn test_different_sessions_have_different_ids() {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();

        let tool_a = claude_bridge_tool(session_a);
        let tool_b = claude_bridge_tool(session_b);

        assert_eq!(tool_a.session_id(), session_a);
        assert_eq!(tool_b.session_id(), session_b);
        assert_ne!(tool_a.session_id(), tool_b.session_id());
    }
}
