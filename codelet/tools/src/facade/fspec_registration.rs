//! FspecTool registration utilities
//!
//! Provides helper functions to create FspecToolFacadeWrapper instances
//! for use in agent builders and tool collections.
//!
//! ## TOOL-012: Session ID at Construction
//!
//! All registration functions accept a `session_id` parameter. This ensures
//! the tool wrapper knows which session's handler to use at call time,
//! eliminating reliance on thread-local current session state.

use super::fspec_facade::{ClaudeFspecFacade, GeminiFspecFacade, OpenAIFspecFacade, ZAIFspecFacade};
use super::wrapper::FspecToolFacadeWrapper;
use std::sync::Arc;
use uuid::Uuid;

/// Create an FspecTool wrapper for Claude provider with explicit session association (TOOL-012)
/// 
/// # Arguments
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn claude_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ClaudeFspecFacade), session_id)
}

/// Create an FspecTool wrapper for Gemini provider with explicit session association (TOOL-012)
/// 
/// # Arguments
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn gemini_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(GeminiFspecFacade), session_id)
}

/// Create an FspecTool wrapper for OpenAI provider with explicit session association (TOOL-012)
/// 
/// # Arguments
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn openai_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(OpenAIFspecFacade), session_id)
}

/// Create an FspecTool wrapper for Z.AI provider with explicit session association (TOOL-012)
/// 
/// # Arguments
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn zai_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ZAIFspecFacade), session_id)
}

/// Create an FspecTool wrapper for Codex provider with explicit session association (TOOL-015)
/// 
/// Codex reuses OpenAIFspecFacade since both use OpenAI-compatible function calling format.
///
/// # Arguments
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn codex_fspec_tool(session_id: Uuid) -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(OpenAIFspecFacade), session_id)
}

/// Create an FspecTool wrapper for the specified provider with explicit session association (TOOL-012)
/// 
/// # Arguments
/// * `provider` - Provider name ("claude", "gemini", "openai", "zai", "codex")
/// * `session_id` - The session ID for handler lookup (must be registered via set_fspec_handler_for_session)
///
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn fspec_tool_for_provider(provider: &str, session_id: Uuid) -> Option<FspecToolFacadeWrapper> {
    match provider {
        "claude" => Some(claude_fspec_tool(session_id)),
        "gemini" => Some(gemini_fspec_tool(session_id)),
        "openai" => Some(openai_fspec_tool(session_id)),
        "zai" => Some(zai_fspec_tool(session_id)),
        "codex" => Some(codex_fspec_tool(session_id)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_provider_tools_created() {
        let session_id = Uuid::new_v4();
        
        let claude = claude_fspec_tool(session_id);
        assert_eq!(claude.provider(), "claude");
        assert_eq!(claude.session_id(), session_id);

        let gemini = gemini_fspec_tool(session_id);
        assert_eq!(gemini.provider(), "gemini");
        assert_eq!(gemini.session_id(), session_id);

        let openai = openai_fspec_tool(session_id);
        assert_eq!(openai.provider(), "openai");
        assert_eq!(openai.session_id(), session_id);

        let zai = zai_fspec_tool(session_id);
        assert_eq!(zai.provider(), "zai");
        assert_eq!(zai.session_id(), session_id);

        let codex = codex_fspec_tool(session_id);
        assert_eq!(codex.provider(), "openai"); // Reuses OpenAIFspecFacade
        assert_eq!(codex.session_id(), session_id);
    }

    #[test]
    fn test_provider_lookup() {
        let session_id = Uuid::new_v4();
        
        assert!(fspec_tool_for_provider("claude", session_id).is_some());
        assert!(fspec_tool_for_provider("gemini", session_id).is_some());
        assert!(fspec_tool_for_provider("openai", session_id).is_some());
        assert!(fspec_tool_for_provider("zai", session_id).is_some());
        assert!(fspec_tool_for_provider("codex", session_id).is_some());
        assert!(fspec_tool_for_provider("unknown", session_id).is_none());
    }

    #[test]
    fn test_different_sessions_have_different_ids() {
        let session_a = Uuid::new_v4();
        let session_b = Uuid::new_v4();
        
        let tool_a = claude_fspec_tool(session_a);
        let tool_b = claude_fspec_tool(session_b);
        
        assert_eq!(tool_a.session_id(), session_a);
        assert_eq!(tool_b.session_id(), session_b);
        assert_ne!(tool_a.session_id(), tool_b.session_id());
    }
}