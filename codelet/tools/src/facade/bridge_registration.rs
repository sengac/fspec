//! BridgeTool registration utilities
//!
//! Provides helper functions to create BridgeToolFacadeWrapper instances
//! for use in agent builders and tool collections.
//!
//! Feature: spec/features/bridge-tool-unit.feature

use super::bridge_facade::{
    ClaudeBridgeFacade, GeminiBridgeFacade, OpenAIBridgeFacade, ZAIBridgeFacade,
};
use super::wrapper::BridgeToolFacadeWrapper;
use std::sync::Arc;

/// Create a BridgeTool wrapper for Claude provider
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn claude_bridge_tool() -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(ClaudeBridgeFacade))
}

/// Create a BridgeTool wrapper for Gemini provider
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn gemini_bridge_tool() -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(GeminiBridgeFacade))
}

/// Create a BridgeTool wrapper for OpenAI provider
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn openai_bridge_tool() -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(OpenAIBridgeFacade))
}

/// Create a BridgeTool wrapper for Z.AI provider
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn zai_bridge_tool() -> BridgeToolFacadeWrapper {
    BridgeToolFacadeWrapper::new(Arc::new(ZAIBridgeFacade))
}

/// Create a BridgeTool wrapper for the specified provider
///
/// NO CLI FALLBACKS - This will throw an error if handler is not configured.
pub fn bridge_tool_for_provider(provider: &str) -> Option<BridgeToolFacadeWrapper> {
    match provider {
        "claude" => Some(claude_bridge_tool()),
        "gemini" => Some(gemini_bridge_tool()),
        "openai" => Some(openai_bridge_tool()),
        "zai" => Some(zai_bridge_tool()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_provider_tools_created() {
        let claude = claude_bridge_tool();
        assert_eq!(claude.provider(), "claude");

        let gemini = gemini_bridge_tool();
        assert_eq!(gemini.provider(), "gemini");

        let openai = openai_bridge_tool();
        assert_eq!(openai.provider(), "openai");

        let zai = zai_bridge_tool();
        assert_eq!(zai.provider(), "zai");
    }

    #[test]
    fn test_provider_lookup() {
        assert!(bridge_tool_for_provider("claude").is_some());
        assert!(bridge_tool_for_provider("gemini").is_some());
        assert!(bridge_tool_for_provider("openai").is_some());
        assert!(bridge_tool_for_provider("zai").is_some());
        assert!(bridge_tool_for_provider("unknown").is_none());
    }
}
