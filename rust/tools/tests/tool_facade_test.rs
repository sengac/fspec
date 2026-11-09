#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/provider-specific-tool-facades.feature
//
// The TOOL-002..006 facade tests live in tool002_*..tool006_*.rs (COV-075).

use anyhow::Result;
use codelet_tools::facade::{
    ClaudeWebSearchFacade, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    InternalWebSearchParams, ProviderToolRegistry, ToolFacade,
};
use serde_json::json;

#[tokio::test]
async fn test_map_claude_web_search_parameters_to_internal_format() -> Result<()> {
    // @step Given a ClaudeWebSearchFacade is registered
    let facade = ClaudeWebSearchFacade;

    // @step When Claude sends parameters {action_type: 'search', query: 'rust async'}
    let claude_params = json!({
        "action_type": "search",
        "query": "rust async"
    });

    // @step Then the facade maps to InternalParams::Search with query 'rust async'
    let internal = facade.map_params(claude_params)?;
    assert_eq!(
        internal,
        InternalWebSearchParams::Search {
            query: "rust async".to_string()
        }
    );

    // @step And the base web search tool executes with the mapped parameters
    // The base tool execution is handled by the existing WebSearchTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_google_web_search_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiGoogleWebSearchFacade is registered
    let facade = GeminiGoogleWebSearchFacade;

    // @step When Gemini sends parameters {query: 'rust async'} to tool 'google_web_search'
    let gemini_params = json!({
        "query": "rust async"
    });

    // @step Then the facade maps to InternalParams::Search with query 'rust async'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalWebSearchParams::Search {
            query: "rust async".to_string()
        }
    );

    // @step And the same base web search tool executes as with Claude
    // Both facades map to the same InternalWebSearchParams::Search type

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_web_fetch_url_to_internal_open_page_format() -> Result<()> {
    // @step Given a GeminiWebFetchFacade is registered
    let facade = GeminiWebFetchFacade;

    // @step When Gemini sends parameters {url: 'https://example.com'} to tool 'web_fetch'
    let gemini_params = json!({
        "url": "https://example.com"
    });

    // @step Then the facade maps to InternalParams::OpenPage with url 'https://example.com'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalWebSearchParams::OpenPage {
            url: "https://example.com".to_string(),
            headless: true,
            pause: false,
        }
    );

    // @step And the base web search tool executes the open_page action
    // The facade correctly passes the URL for the base tool

    Ok(())
}

#[tokio::test]
async fn test_registry_returns_only_facades_for_requested_provider() -> Result<()> {
    // @step Given facades are registered for both Claude and Gemini providers
    let registry = ProviderToolRegistry::new();

    // @step When I request tools_for_provider('gemini')
    let gemini_tools = registry.tools_for_provider("gemini");

    // @step Then the registry returns GeminiGoogleWebSearchFacade and GeminiWebFetchFacade
    let tool_names: Vec<&str> = gemini_tools.iter().map(|t| t.tool_name()).collect();
    assert!(tool_names.contains(&"google_web_search"));
    assert!(tool_names.contains(&"web_fetch"));

    // @step And the registry does not return any Claude facades
    assert!(!tool_names.contains(&"web_search"));
    for tool in &gemini_tools {
        assert_eq!(tool.provider(), "gemini");
    }

    Ok(())
}

#[tokio::test]
async fn test_gemini_facade_provides_flat_json_schema_without_oneof() -> Result<()> {
    // @step Given a GeminiGoogleWebSearchFacade is created
    let facade = GeminiGoogleWebSearchFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing only {query: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["query"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_claude_facade_provides_flat_schema_with_action_type_enum() -> Result<()> {
    // @step Given a ClaudeWebSearchFacade is created
    let facade = ClaudeWebSearchFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema contains an 'action_type' property with enum values
    let params = &definition.parameters;
    assert!(params["properties"]["action_type"]["enum"].is_array());

    // @step And the enum values include search, open_page, and find_in_page action types
    let action_types = params["properties"]["action_type"]["enum"]
        .as_array()
        .unwrap();
    let types: Vec<&str> = action_types.iter().filter_map(|v| v.as_str()).collect();
    assert!(types.contains(&"search"));
    assert!(types.contains(&"open_page"));
    assert!(types.contains(&"find_in_page"));

    Ok(())
}
