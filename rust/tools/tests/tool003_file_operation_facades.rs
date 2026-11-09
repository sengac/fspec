#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/file-operation-facades.feature
//
// TOOL-003 file operation facade tests, moved out of tool_facade_test.rs (COV-075: one test file per feature).

use anyhow::Result;
use codelet_tools::facade::{
    FileToolFacade, FileToolFacadeWrapper, GeminiReadFileFacade, GeminiReplaceFacade,
    GeminiWriteFileFacade, InternalFileParams,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// File Operation Facades Tests (TOOL-003)
// Feature: spec/features/file-operation-facades.feature
// ============================================================================

#[tokio::test]
async fn test_map_gemini_read_file_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiReadFileFacade is registered
    let facade = GeminiReadFileFacade;

    // @step When Gemini sends parameters {file_path: '/tmp/file.txt'} to tool 'read_file'
    let gemini_params = json!({
        "file_path": "/tmp/file.txt"
    });

    // @step Then the facade maps to InternalFileParams::Read with file_path '/tmp/file.txt'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalFileParams::Read {
            file_path: "/tmp/file.txt".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );

    // @step And the same base ReadTool executes with the mapped parameters
    // The base tool execution is handled by the existing ReadTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_write_file_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiWriteFileFacade is registered
    let facade = GeminiWriteFileFacade;

    // @step When Gemini sends parameters {file_path: '/tmp/file.txt', content: 'hello'} to tool 'write_file'
    let gemini_params = json!({
        "file_path": "/tmp/file.txt",
        "content": "hello"
    });

    // @step Then the facade maps to InternalFileParams::Write with file_path '/tmp/file.txt' and content 'hello'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalFileParams::Write {
            file_path: "/tmp/file.txt".to_string(),
            content: "hello".to_string(),
        }
    );

    // @step And the same base WriteTool executes with the mapped parameters

    Ok(())
}

#[tokio::test]
async fn test_map_gemini_replace_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiReplaceFacade is registered
    let facade = GeminiReplaceFacade;

    // @step When Gemini sends parameters {file_path: '/tmp/file.txt', old_string: 'foo', new_string: 'bar'} to tool 'replace'
    let gemini_params = json!({
        "file_path": "/tmp/file.txt",
        "old_string": "foo",
        "new_string": "bar"
    });

    // @step Then the facade maps to InternalFileParams::Edit with file_path '/tmp/file.txt', old_string 'foo', new_string 'bar'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalFileParams::Edit {
            file_path: "/tmp/file.txt".to_string(),
            old_string: "foo".to_string(),
            new_string: "bar".to_string(),
        }
    );

    // @step And the same base EditTool executes with the mapped parameters

    Ok(())
}

#[tokio::test]
async fn test_gemini_read_file_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiReadFileFacade is created
    let facade = GeminiReadFileFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {file_path: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["file_path"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_gemini_write_file_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiWriteFileFacade is created
    let facade = GeminiWriteFileFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {file_path: {type: 'string'}, content: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["file_path"]["type"] == "string");
    assert!(params["properties"]["content"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_gemini_replace_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiReplaceFacade is created
    let facade = GeminiReplaceFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {file_path: {type: 'string'}, old_string: {type: 'string'}, new_string: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["file_path"]["type"] == "string");
    assert!(params["properties"]["old_string"]["type"] == "string");
    assert!(params["properties"]["new_string"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_facade_wrapper_read_file_integrates_with_rig() -> Result<()> {
    // @step Given a FileToolFacadeWrapper wrapping GeminiReadFileFacade
    let facade = Arc::new(GeminiReadFileFacade) as Arc<dyn FileToolFacade>;
    // BLOCK-006: FileToolFacadeWrapper now requires session_id (use nil for tests)
    let wrapper = FileToolFacadeWrapper::new(facade, Uuid::nil());

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "read_file"
    assert_eq!(name, "read_file");

    // @step And when I call definition() it returns a flat schema with file_path parameter
    let def = wrapper.definition(String::new()).await;
    assert_eq!(def.name, "read_file");
    assert!(def.parameters["properties"]["file_path"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());

    Ok(())
}

#[tokio::test]
async fn test_file_facades_available_for_gemini_provider() -> Result<()> {
    // @step Given a GeminiProvider is configured
    // File facades are created separately from web search facades
    let read_facade = GeminiReadFileFacade;
    let write_facade = GeminiWriteFileFacade;
    let replace_facade = GeminiReplaceFacade;

    // @step When create_rig_agent() is called
    // The facades are wrapped with FileToolFacadeWrapper for rig integration
    // BLOCK-006: FileToolFacadeWrapper now requires session_id (use nil for tests)
    let read_wrapper = FileToolFacadeWrapper::new(
        Arc::new(read_facade) as Arc<dyn FileToolFacade>,
        Uuid::nil(),
    );
    let write_wrapper = FileToolFacadeWrapper::new(
        Arc::new(write_facade) as Arc<dyn FileToolFacade>,
        Uuid::nil(),
    );
    let replace_wrapper = FileToolFacadeWrapper::new(
        Arc::new(replace_facade) as Arc<dyn FileToolFacade>,
        Uuid::nil(),
    );

    // @step Then the agent has tool 'read_file' backed by GeminiReadFileFacade
    assert_eq!(read_wrapper.name(), "read_file");

    // @step And the agent has tool 'write_file' backed by GeminiWriteFileFacade
    assert_eq!(write_wrapper.name(), "write_file");

    // @step And the agent has tool 'replace' backed by GeminiReplaceFacade
    assert_eq!(replace_wrapper.name(), "replace");

    // @step And all file operation tools use FacadeToolWrapper instead of raw tools
    // All facades report "gemini" as their provider
    assert_eq!(read_wrapper.provider(), "gemini");
    assert_eq!(write_wrapper.provider(), "gemini");
    assert_eq!(replace_wrapper.provider(), "gemini");

    Ok(())
}
