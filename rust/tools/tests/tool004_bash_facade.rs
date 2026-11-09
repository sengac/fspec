#![allow(clippy::unwrap_used, clippy::expect_used)]
// Feature: spec/features/bash-facade.feature
//
// TOOL-004 bash facade tests, moved out of tool_facade_test.rs (COV-075: one test file per feature).

use anyhow::Result;
use codelet_tools::facade::{
    BashToolFacade, BashToolFacadeWrapper, GeminiRunShellCommandFacade, InternalBashParams,
};
use rig::tool::Tool;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

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
            command: "ls -la".to_string(),
            cwd: None,
            timeout_ms: None,
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
    // BLOCK-006: BashToolFacadeWrapper now requires session_id (use nil for tests)
    let wrapper = BashToolFacadeWrapper::new(facade, Uuid::nil());

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
