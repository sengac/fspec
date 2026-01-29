//! FspecTool registration utilities
//!
//! Provides helper functions to create FspecToolFacadeWrapper instances
//! for use in agent builders and tool collections.

use super::fspec_facade::{ClaudeFspecFacade, GeminiFspecFacade, OpenAIFspecFacade, ZAIFspecFacade};
use super::wrapper::FspecToolFacadeWrapper;
use std::sync::Arc;

/// Create an FspecTool wrapper for Claude provider
/// 
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn claude_fspec_tool() -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ClaudeFspecFacade))
}

/// Create an FspecTool wrapper for Gemini provider
/// 
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn gemini_fspec_tool() -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(GeminiFspecFacade))
}

/// Create an FspecTool wrapper for OpenAI provider
/// 
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn openai_fspec_tool() -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(OpenAIFspecFacade))
}

/// Create an FspecTool wrapper for Z.AI provider
/// 
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn zai_fspec_tool() -> FspecToolFacadeWrapper {
    FspecToolFacadeWrapper::new(Arc::new(ZAIFspecFacade))
}

/// Create an FspecTool wrapper for the specified provider
/// 
/// NO CLI FALLBACKS - This will throw an error if callback system is not working.
pub fn fspec_tool_for_provider(provider: &str) -> Option<FspecToolFacadeWrapper> {
    match provider {
        "claude" => Some(claude_fspec_tool()),
        "gemini" => Some(gemini_fspec_tool()),
        "openai" => Some(openai_fspec_tool()),
        "zai" => Some(zai_fspec_tool()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_provider_tools_created() {
        let claude = claude_fspec_tool();
        assert_eq!(claude.provider(), "claude");

        let gemini = gemini_fspec_tool();
        assert_eq!(gemini.provider(), "gemini");

        let openai = openai_fspec_tool();
        assert_eq!(openai.provider(), "openai");

        let zai = zai_fspec_tool();
        assert_eq!(zai.provider(), "zai");
    }

    #[test]
    fn test_provider_lookup() {
        assert!(fspec_tool_for_provider("claude").is_some());
        assert!(fspec_tool_for_provider("gemini").is_some());
        assert!(fspec_tool_for_provider("openai").is_some());
        assert!(fspec_tool_for_provider("zai").is_some());
        assert!(fspec_tool_for_provider("unknown").is_none());
    }
}