#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/search-facades.feature
//
// TOOL-005 search facade tests, moved out of tool_facade_test.rs (COV-075: one test file per feature).

use anyhow::Result;
use codelet_tools::facade::{
    GeminiGlobFacade, GeminiSearchFileContentFacade, InternalSearchParams, SearchToolFacade,
    SearchToolFacadeWrapper,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Search Facades Tests (TOOL-005)
// Feature: spec/features/search-facades.feature
// ============================================================================

#[tokio::test]
async fn test_map_gemini_search_file_content_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiSearchFileContentFacade is registered
    let facade = GeminiSearchFileContentFacade;

    // @step When Gemini sends parameters {pattern: 'TODO', dir_path: 'src'} to tool 'search_file_content'
    let gemini_params = json!({
        "pattern": "TODO",
        "dir_path": "src"
    });

    // @step Then the facade maps to InternalSearchParams::Grep with pattern 'TODO' and path 'src'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: Some("src".to_string()),
            include: None,
            limit: None,
        }
    );

    // @step And the base GrepTool executes with the mapped parameters
    // The base tool execution is handled by the existing GrepTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_gemini_search_file_content_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiSearchFileContentFacade is created
    let facade = GeminiSearchFileContentFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {pattern: {type: 'string'}, dir_path: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["pattern"]["type"] == "string");
    assert!(params["properties"]["dir_path"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_glob_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiGlobFacade is registered
    let facade = GeminiGlobFacade;

    // The pattern is passed through as-is; '*.rs' keeps the @step text free
    // of "*/", which the step matcher cuts at (BUG-499).
    // @step When Gemini sends parameters {pattern: '*.rs', dir_path: 'src'} to tool 'glob'
    let gemini_params = json!({
        "pattern": "*.rs",
        "dir_path": "src"
    });

    // @step Then the facade maps to InternalSearchParams::Glob with pattern '*.rs' and path 'src'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalSearchParams::Glob {
            pattern: "*.rs".to_string(),
            path: Some("src".to_string())
        }
    );

    // @step And the base GlobTool executes with the mapped parameters
    // The base tool execution is handled by the existing GlobTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_gemini_glob_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiGlobFacade is created
    let facade = GeminiGlobFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {pattern: {type: 'string'}, dir_path: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["pattern"]["type"] == "string");
    assert!(params["properties"]["dir_path"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_search_facade_wrapper_integrates_with_rig_for_search_file_content() -> Result<()> {
    // @step Given a SearchToolFacadeWrapper wrapping GeminiSearchFileContentFacade
    let facade = Arc::new(GeminiSearchFileContentFacade) as Arc<dyn SearchToolFacade>;
    let wrapper = SearchToolFacadeWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "search_file_content"
    assert_eq!(name, "search_file_content");

    // @step And when I call definition() it returns a flat schema with pattern and dir_path parameters
    let def = wrapper.definition(String::new()).await;
    assert_eq!(def.name, "search_file_content");
    assert!(def.parameters["properties"]["pattern"]["type"] == "string");
    assert!(def.parameters["properties"]["dir_path"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());

    Ok(())
}

#[tokio::test]
async fn test_search_facade_wrapper_integrates_with_rig_for_glob() -> Result<()> {
    // @step Given a SearchToolFacadeWrapper wrapping GeminiGlobFacade
    let facade = Arc::new(GeminiGlobFacade) as Arc<dyn SearchToolFacade>;
    let wrapper = SearchToolFacadeWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "glob"
    assert_eq!(name, "glob");

    // @step And when I call definition() it returns a flat schema with pattern and dir_path parameters
    let def = wrapper.definition(String::new()).await;
    assert_eq!(def.name, "glob");
    assert!(def.parameters["properties"]["pattern"]["type"] == "string");
    assert!(def.parameters["properties"]["dir_path"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());

    Ok(())
}
