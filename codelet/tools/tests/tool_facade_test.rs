// Feature: spec/features/provider-specific-tool-facades.feature

use anyhow::Result;
use codelet_tools::facade::{
    ClaudeWebSearchFacade, FacadeToolWrapper, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade,
    InternalWebSearchParams, ProviderToolRegistry, ToolFacade,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_map_claude_web_search_parameters_to_internal_format() -> Result<()> {
    // @step Given a ClaudeWebSearchFacade is registered
    let facade = ClaudeWebSearchFacade;

    // @step When Claude sends parameters {action: {type: 'search', query: 'rust async'}}
    let claude_params = json!({
        "action": {
            "type": "search",
            "query": "rust async"
        }
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

    // @step When Gemini sends parameters {prompt: 'https://example.com summarize this'} to tool 'web_fetch'
    let gemini_params = json!({
        "prompt": "https://example.com summarize this"
    });

    // @step Then the facade extracts the URL and maps to InternalParams::OpenPage with url 'https://example.com'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalWebSearchParams::OpenPage {
            url: "https://example.com".to_string()
        }
    );

    // @step And the base web search tool executes the open_page action
    // The facade correctly extracts the URL for the base tool

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
async fn test_claude_facade_provides_complex_schema_with_action_variants() -> Result<()> {
    // @step Given a ClaudeWebSearchFacade is created
    let facade = ClaudeWebSearchFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema contains an 'action' property with 'oneOf' variants
    let params = &definition.parameters;
    assert!(params["properties"]["action"]["oneOf"].is_array());

    // @step And the variants include search, open_page, and find_in_page action types
    let one_of = params["properties"]["action"]["oneOf"].as_array().unwrap();
    let types: Vec<&str> = one_of
        .iter()
        .filter_map(|v| v["properties"]["type"]["const"].as_str())
        .collect();
    assert!(types.contains(&"search"));
    assert!(types.contains(&"open_page"));
    assert!(types.contains(&"find_in_page"));

    Ok(())
}

/// Test that FacadeToolWrapper correctly implements rig::tool::Tool trait
/// This verifies the wrapper provides Gemini-native tool names and schemas
#[tokio::test]
async fn test_facade_wrapper_integrates_with_rig_tool_trait() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    let facade = Arc::new(GeminiGoogleWebSearchFacade);
    let wrapper = FacadeToolWrapper::new(facade);

    // @step When I call name() on the wrapper (rig::tool::Tool method)
    let name = wrapper.name();

    // @step Then it returns "google_web_search" (Gemini-native name)
    assert_eq!(name, "google_web_search");

    // @step And when I call definition() (rig::tool::Tool method)
    let def = wrapper.definition(String::new()).await;

    // @step Then it returns a flat schema without oneOf
    assert_eq!(def.name, "google_web_search");
    assert!(def.parameters["properties"]["query"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());
    assert!(def.parameters["properties"].get("action").is_none());

    Ok(())
}

/// Test that web_fetch wrapper also works correctly
#[tokio::test]
async fn test_facade_wrapper_web_fetch_integrates_with_rig() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiWebFetchFacade
    let facade = Arc::new(GeminiWebFetchFacade);
    let wrapper = FacadeToolWrapper::new(facade);

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "web_fetch" (Gemini-native name)
    assert_eq!(name, "web_fetch");

    // @step And when I call definition()
    let def = wrapper.definition(String::new()).await;

    // @step Then it returns a flat schema with prompt parameter
    assert_eq!(def.name, "web_fetch");
    assert!(def.parameters["properties"]["prompt"]["type"] == "string");

    Ok(())
}
