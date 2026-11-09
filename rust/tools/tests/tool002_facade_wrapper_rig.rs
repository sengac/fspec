#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/facadetoolwrapper-for-rig-integration.feature
//
// TOOL-002 FacadeToolWrapper tests, moved out of tool_facade_test.rs (COV-075: one test file per feature).

use anyhow::Result;
use codelet_tools::facade::{FacadeToolWrapper, GeminiGoogleWebSearchFacade, GeminiWebFetchFacade};
use rig::tool::Tool;
use std::sync::Arc;
use uuid::Uuid;

/// Test that FacadeToolWrapper correctly implements rig::tool::Tool trait
/// This verifies the wrapper provides Gemini-native tool names and schemas
#[tokio::test]
#[ignore = "Spawns Chrome via WebSearchTool - run with --ignored flag"]
async fn test_facade_wrapper_integrates_with_rig_tool_trait() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    let facade = Arc::new(GeminiGoogleWebSearchFacade);
    let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "google_web_search"
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

// Feature: spec/features/facadetoolwrapper-for-rig-integration.feature
// Scenario: FacadeToolWrapper returns facade definition with flat schema
#[tokio::test]
#[ignore = "Spawns Chrome via WebSearchTool - run with --ignored flag"]
async fn test_facade_wrapper_returns_definition_with_flat_schema() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    let facade = Arc::new(GeminiGoogleWebSearchFacade);
    let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

    // @step When I call definition() on the wrapper
    let def = wrapper.definition(String::new()).await;

    // @step Then it returns a flat schema without oneOf
    assert!(def.parameters.get("oneOf").is_none());
    assert!(def.parameters["properties"].get("action").is_none());

    // @step And the schema has name "google_web_search"
    assert_eq!(def.name, "google_web_search");

    Ok(())
}

/// Test that web_fetch wrapper also works correctly
#[tokio::test]
#[ignore = "Spawns Chrome via WebSearchTool - run with --ignored flag"]
async fn test_facade_wrapper_web_fetch_integrates_with_rig() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiWebFetchFacade
    let facade = Arc::new(GeminiWebFetchFacade);
    let wrapper = FacadeToolWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "web_fetch"
    assert_eq!(name, "web_fetch");

    // @step And when I call definition()
    let def = wrapper.definition(String::new()).await;

    // @step Then it returns a flat schema with url and format parameters
    assert_eq!(def.name, "web_fetch");
    assert!(def.parameters["properties"]["url"]["type"] == "string");
    assert!(def.parameters["properties"]["format"]["type"] == "string");
    assert!(def.parameters["properties"]["format"]["enum"].is_array());

    Ok(())
}
