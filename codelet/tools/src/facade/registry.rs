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
mod tests {
    use super::*;

    #[test]
    fn test_registry_returns_claude_facades() {
        let registry = ProviderToolRegistry::new();
        let tools = registry.tools_for_provider("claude");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].tool_name(), "web_search");
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
    fn test_registry_does_not_mix_providers() {
        let registry = ProviderToolRegistry::new();
        let gemini_tools = registry.tools_for_provider("gemini");

        // Gemini tools should not include Claude's web_search
        let tool_names: Vec<&str> = gemini_tools.iter().map(|t| t.tool_name()).collect();
        assert!(!tool_names.contains(&"web_search"));
    }
}
