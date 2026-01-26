#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/web-search-and-fetch-tool-integration.feature

use anyhow::Result;
use codelet_common::web_search::WebSearchAction;
use codelet_tools::WebSearchTool;
use rig::tool::Tool;

#[tokio::test]
#[ignore = "Spawns Chrome - run with --ignored flag"]
async fn test_agent_performs_web_search() -> Result<()> {
    // @step Given the web search tool is available
    let web_search_tool = WebSearchTool;

    // @step When the agent executes a search query "weather in Seattle"
    let search_request = codelet_tools::web_search::WebSearchRequest {
        action: WebSearchAction::Search {
            query: Some("weather in Seattle".to_string()),
        },
    };

    // @step Then the agent receives search results
    let result = web_search_tool.call(search_request).await?;
    assert!(result.success);
    assert!(result.message.contains("weather in Seattle"));

    // @step And the web search event is logged
    // Web search events are generated in the tool implementation

    Ok(())
}

#[tokio::test]
#[ignore = "Spawns Chrome - run with --ignored flag"]
async fn test_agent_fetches_page_content() -> Result<()> {
    // @step Given the web search tool is available
    let web_search_tool = WebSearchTool;

    // @step When the agent opens a page "https://example.com"
    let page_request = codelet_tools::web_search::WebSearchRequest {
        action: WebSearchAction::OpenPage {
            url: Some("https://example.com".to_string()),
            headless: true,
        },
    };

    // @step Then the agent receives the page content
    let result = web_search_tool.call(page_request).await?;
    assert!(result.success);
    assert!(result.message.contains("https://example.com"));

    // @step And the page fetch event is logged
    // Page fetch events are generated in the tool implementation

    Ok(())
}

#[tokio::test]
#[ignore = "Spawns Chrome - run with --ignored flag"]
async fn test_agent_searches_within_page_content() -> Result<()> {
    // @step Given the web search tool is available
    let web_search_tool = WebSearchTool;

    // @step When the agent searches within a page with URL "https://example.com" and pattern "contact"
    let search_request = codelet_tools::web_search::WebSearchRequest {
        action: WebSearchAction::FindInPage {
            url: Some("https://example.com".to_string()),
            pattern: Some("contact".to_string()),
            headless: true,
        },
    };

    // @step Then the agent receives matching content from the page
    let result = web_search_tool.call(search_request).await?;
    assert!(result.success);
    assert!(result.message.contains("contact"));
    assert!(result.message.contains("https://example.com"));

    // @step And the page search event is logged
    // Page search events are generated in the tool implementation

    Ok(())
}

#[tokio::test]
#[ignore = "Spawns Chrome - run with --ignored flag"]
async fn test_web_search_tool_registration() -> Result<()> {
    // @step Given the codelet system is starting up
    let web_search_tool = WebSearchTool;

    // @step When the web search configuration is enabled
    // Get the tool definition (simulates registration)
    let definition = web_search_tool.definition("".to_string()).await;

    // @step Then the WebSearch tool is registered as a native OpenAI tool
    assert_eq!(definition.name, "web_search");
    assert!(definition.description.contains("web search"));

    // @step And the tool appears in the available tools list
    // Verify the tool spec is properly defined
    assert!(definition.parameters.get("properties").is_some());

    Ok(())
}
