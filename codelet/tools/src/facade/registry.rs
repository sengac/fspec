//! Provider Tool Registry for managing tool facades.

use super::traits::BoxedToolFacade;
use super::web_search::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    GeminiWebScreenshotFacade,
};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry for provider-specific tool facades.
///
/// The registry manages facades for different providers, allowing each provider
/// to receive only the tools configured for their expected schemas and naming.
pub struct ProviderToolRegistry {
    /// Map of (provider, tool_name) -> facade
    facades: HashMap<(&'static str, &'static str), BoxedToolFacade>,
}

impl Default for ProviderToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderToolRegistry {
    /// Creates a new registry with all registered facades.
    pub fn new() -> Self {
        let mut facades = HashMap::new();

        // Register Claude facades
        let claude_web_search = Arc::new(ClaudeWebSearchFacade) as BoxedToolFacade;
        facades.insert(
            (claude_web_search.provider(), claude_web_search.tool_name()),
            claude_web_search,
        );

        // Register Gemini facades
        let gemini_web_search = Arc::new(GeminiGoogleWebSearchFacade) as BoxedToolFacade;
        facades.insert(
            (gemini_web_search.provider(), gemini_web_search.tool_name()),
            gemini_web_search,
        );

        let gemini_web_fetch = Arc::new(GeminiWebFetchFacade) as BoxedToolFacade;
        facades.insert(
            (gemini_web_fetch.provider(), gemini_web_fetch.tool_name()),
            gemini_web_fetch,
        );

        let gemini_web_screenshot = Arc::new(GeminiWebScreenshotFacade) as BoxedToolFacade;
        facades.insert(
            (
                gemini_web_screenshot.provider(),
                gemini_web_screenshot.tool_name(),
            ),
            gemini_web_screenshot,
        );

        // Note: FspecTool is handled separately via FspecToolFacadeWrapper
        // It's not registered here because it doesn't use web search patterns
        
        Self { facades }
    }

    /// Returns all facades registered for the specified provider.
    pub fn tools_for_provider(&self, provider: &str) -> Vec<BoxedToolFacade> {
        self.facades
            .iter()
            .filter(|((p, _), _)| *p == provider)
            .map(|(_, facade)| Arc::clone(facade))
            .collect()
    }

    /// Returns a specific facade by provider and tool name.
    pub fn get_facade(&self, provider: &str, tool_name: &str) -> Option<BoxedToolFacade> {
        // Need to find the key that matches
        for ((p, t), facade) in &self.facades {
            if *p == provider && *t == tool_name {
                return Some(Arc::clone(facade));
            }
        }
        None
    }

    /// Returns all tool definitions for a provider.
    pub fn definitions_for_provider(&self, provider: &str) -> Vec<super::ToolDefinition> {
        self.tools_for_provider(provider)
            .into_iter()
            .map(|f| f.definition())
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_returns_claude_facades() {
        let registry = ProviderToolRegistry::new();
        let tools = registry.tools_for_provider("claude");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_name(), "WebSearch");
        assert_eq!(tools[0].provider(), "claude");
    }

    #[test]
    fn test_registry_returns_gemini_facades() {
        let registry = ProviderToolRegistry::new();
        let tools = registry.tools_for_provider("gemini");

        assert_eq!(tools.len(), 3);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.tool_name()).collect();
        assert!(tool_names.contains(&"google_web_search"));
        assert!(tool_names.contains(&"web_fetch"));
        assert!(tool_names.contains(&"capture_screenshot"));

        // All should be for gemini provider
        for tool in &tools {
            assert_eq!(tool.provider(), "gemini");
        }
    }

    #[test]
    fn test_fspec_tool_wrappers_created_separately() {
        use super::super::fspec_registration::{
            claude_fspec_tool, gemini_fspec_tool, openai_fspec_tool, zai_fspec_tool,
        };

        // Test that FspecToolFacadeWrapper can be created for all providers
        let claude_wrapper = claude_fspec_tool();
        assert_eq!(claude_wrapper.provider(), "claude");

        let gemini_wrapper = gemini_fspec_tool();
        assert_eq!(gemini_wrapper.provider(), "gemini");

        let openai_wrapper = openai_fspec_tool();
        assert_eq!(openai_wrapper.provider(), "openai");

        let zai_wrapper = zai_fspec_tool();
        assert_eq!(zai_wrapper.provider(), "zai");
    }

    #[test]
    fn test_registry_does_not_mix_providers() {
        let registry = ProviderToolRegistry::new();
        let gemini_tools = registry.tools_for_provider("gemini");

        // Gemini tools should not include Claude's web_search
        assert!(!gemini_tools
            .iter()
            .map(|t| t.tool_name())
            .any(|name| name == "web_search"));
    }
}