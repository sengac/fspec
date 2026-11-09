#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/directory-listing-facade.feature
//
// TOOL-006 directory listing facade tests, moved out of tool_facade_test.rs (COV-075: one test file per feature).

use anyhow::Result;
use codelet_tools::facade::{
    GeminiListDirectoryFacade, InternalLsParams, LsToolFacade, LsToolFacadeWrapper,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// Directory Listing Facade Tests (TOOL-006)
// Feature: spec/features/directory-listing-facade.feature
// ============================================================================

#[tokio::test]
async fn test_map_gemini_list_directory_parameters_with_path() -> Result<()> {
    // @step Given a GeminiListDirectoryFacade is registered
    let facade = GeminiListDirectoryFacade;

    // @step When Gemini sends parameters {path: 'src'} to tool 'list_directory'
    let gemini_params = json!({
        "path": "src"
    });

    // @step Then the facade maps to InternalLsParams::List with path 'src'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalLsParams::List {
            path: Some("src".to_string()),
            offset: None,
            limit: None,
            depth: None,
        }
    );

    // @step And the base LsTool executes with the mapped parameters
    // The base tool execution is handled by the existing LsTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_list_directory_with_empty_parameters() -> Result<()> {
    // @step Given a GeminiListDirectoryFacade is registered
    let facade = GeminiListDirectoryFacade;

    // @step When Gemini sends empty parameters {} to tool 'list_directory'
    let gemini_params = json!({});

    // @step Then the facade maps to InternalLsParams::List with path None
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalLsParams::List {
            path: None,
            offset: None,
            limit: None,
            depth: None
        }
    );

    // @step And the base LsTool lists the current directory
    // The base tool execution is handled by the existing LsTool

    Ok(())
}

#[tokio::test]
async fn test_gemini_list_directory_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiListDirectoryFacade is created
    let facade = GeminiListDirectoryFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {path: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["path"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    // @step And the 'path' parameter is optional (not in required array)
    let required = params
        .get("required")
        .and_then(|r: &serde_json::Value| r.as_array());
    assert!(required.is_none() || required.unwrap().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_ls_tool_facade_wrapper_integrates_with_rig() -> Result<()> {
    // @step Given a LsToolFacadeWrapper wrapping GeminiListDirectoryFacade
    let facade = Arc::new(GeminiListDirectoryFacade) as Arc<dyn LsToolFacade>;
    let wrapper = LsToolFacadeWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "list_directory"
    assert_eq!(name, "list_directory");

    // @step And when I call definition() it returns a flat schema with path parameter
    let def = wrapper.definition(String::new()).await;
    assert_eq!(def.name, "list_directory");
    assert!(def.parameters["properties"]["path"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());

    Ok(())
}
