#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/provider-specific-tool-facades.feature
// Feature: spec/features/file-operation-facades.feature
// Feature: spec/features/bash-facade.feature
// Feature: spec/features/search-facades.feature
// Feature: spec/features/directory-listing-facade.feature

use anyhow::Result;
use codelet_tools::facade::{
    BashToolFacade, BashToolFacadeWrapper, ClaudeWebSearchFacade, FacadeToolWrapper,
    FileToolFacade, FileToolFacadeWrapper, GeminiGlobFacade, GeminiGoogleWebSearchFacade,
    GeminiListDirectoryFacade, GeminiReadFileFacade, GeminiReplaceFacade,
    GeminiRunShellCommandFacade, GeminiSearchFileContentFacade, GeminiWebFetchFacade,
    GeminiWriteFileFacade, InternalBashParams, InternalFileParams, InternalLsParams,
    InternalSearchParams, InternalWebSearchParams, LsToolFacade, LsToolFacadeWrapper,
    ProviderToolRegistry, SearchToolFacade, SearchToolFacadeWrapper, ToolFacade,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;

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
            url: "https://example.com".to_string()
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

/// Test that FacadeToolWrapper correctly implements rig::tool::Tool trait
/// This verifies the wrapper provides Gemini-native tool names and schemas
#[tokio::test]
#[ignore = "Spawns Chrome via WebSearchTool - run with --ignored flag"]
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

// Feature: spec/features/facadetoolwrapper-for-rig-integration.feature
// Scenario: FacadeToolWrapper returns facade definition with flat schema
#[tokio::test]
#[ignore = "Spawns Chrome via WebSearchTool - run with --ignored flag"]
async fn test_facade_wrapper_returns_definition_with_flat_schema() -> Result<()> {
    // @step Given a FacadeToolWrapper wrapping GeminiGoogleWebSearchFacade
    let facade = Arc::new(GeminiGoogleWebSearchFacade);
    let wrapper = FacadeToolWrapper::new(facade);

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
    let wrapper = FacadeToolWrapper::new(facade);

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "web_fetch" (Gemini-native name)
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
    let wrapper = FileToolFacadeWrapper::new(facade);

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
    let read_wrapper = FileToolFacadeWrapper::new(Arc::new(read_facade) as Arc<dyn FileToolFacade>);
    let write_wrapper =
        FileToolFacadeWrapper::new(Arc::new(write_facade) as Arc<dyn FileToolFacade>);
    let replace_wrapper =
        FileToolFacadeWrapper::new(Arc::new(replace_facade) as Arc<dyn FileToolFacade>);

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

// ============================================================================
// Bash Facade Tests (TOOL-004)
// Feature: spec/features/bash-facade.feature
// ============================================================================

#[tokio::test]
async fn test_map_gemini_run_shell_command_parameters_to_internal_format() -> Result<()> {
    // @step Given a GeminiRunShellCommandFacade is registered
    let facade = GeminiRunShellCommandFacade;

    // @step When Gemini sends parameters {command: 'ls -la'} to tool 'run_shell_command'
    let gemini_params = json!({
        "command": "ls -la"
    });

    // @step Then the facade maps to BashArgs with command 'ls -la'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalBashParams::Execute {
            command: "ls -la".to_string()
        }
    );

    // @step And the base BashTool executes with the mapped parameters
    // The base tool execution is handled by the existing BashTool
    // This test verifies the facade correctly maps the parameters

    Ok(())
}

#[tokio::test]
async fn test_gemini_run_shell_command_facade_provides_flat_schema() -> Result<()> {
    // @step Given a GeminiRunShellCommandFacade is created
    let facade = GeminiRunShellCommandFacade;

    // @step When I request the tool definition
    let definition = facade.definition();

    // @step Then the schema has type 'object' with properties containing {command: {type: 'string'}}
    let params = &definition.parameters;
    assert_eq!(params["type"], "object");
    assert!(params["properties"]["command"]["type"] == "string");

    // @step And the schema does not contain 'oneOf' or nested action objects
    assert!(params.get("oneOf").is_none());
    assert!(params["properties"].get("action").is_none());

    Ok(())
}

#[tokio::test]
async fn test_bash_facade_wrapper_integrates_with_rig() -> Result<()> {
    // @step Given a BashToolFacadeWrapper wrapping GeminiRunShellCommandFacade
    let facade = Arc::new(GeminiRunShellCommandFacade) as Arc<dyn BashToolFacade>;
    let wrapper = BashToolFacadeWrapper::new(facade);

    // @step When I call name() on the wrapper
    let name = wrapper.name();

    // @step Then it returns "run_shell_command"
    assert_eq!(name, "run_shell_command");

    // @step And when I call definition() it returns a flat schema with command parameter
    let def = wrapper.definition(String::new()).await;
    assert_eq!(def.name, "run_shell_command");
    assert!(def.parameters["properties"]["command"]["type"] == "string");
    assert!(def.parameters.get("oneOf").is_none());

    Ok(())
}

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
            path: Some("src".to_string())
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

    // @step When Gemini sends parameters {pattern: '**/*.rs', dir_path: 'src'} to tool 'glob'
    let gemini_params = json!({
        "pattern": "**/*.rs",
        "dir_path": "src"
    });

    // @step Then the facade maps to InternalSearchParams::Glob with pattern '**/*.rs' and path 'src'
    let internal = facade.map_params(gemini_params)?;
    assert_eq!(
        internal,
        InternalSearchParams::Glob {
            pattern: "**/*.rs".to_string(),
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
    let wrapper = SearchToolFacadeWrapper::new(facade);

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
    let wrapper = SearchToolFacadeWrapper::new(facade);

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
            path: Some("src".to_string())
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
    assert_eq!(internal, InternalLsParams::List { path: None });

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
    let wrapper = LsToolFacadeWrapper::new(facade);

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
