//! Feature: spec/features/codex-shell-exec-facades.feature
//!
//! BUG-114 exec_command facade tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// BUG-114: exec_command facade tests (ExecToolFacade)
// Feature: spec/features/codex-shell-exec-facades.feature
// =========================================================================

/// Scenario: CodexExecCommandFacade maps exec_command with PTY to InternalExecParams::Run
#[test]
fn test_codex_exec_command_facade_maps_pty() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the Codex model calls exec_command with cmd "python3" tty true and yield_time_ms 5000
    let input = json!({
        "cmd": "python3",
        "tty": true,
        "yield_time_ms": 5000
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with command as string "python3"
    // @step And tty is true
    // @step And yield_time_ms is 5000
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: Value::String("python3".to_string()),
            workdir: None,
            tty: true,
            yield_time_ms: Some(5000),
            max_output_tokens: None,
            timeout_secs: None,
        }
    );

    // @step And the facade tool name is "exec_command"
    assert_eq!(facade.tool_name(), "exec_command");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: CodexExecCommandFacade defaults to tty false when not specified
#[test]
fn test_codex_exec_command_facade_defaults_tty_false() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the Codex model calls exec_command with only cmd "ls"
    let input = json!({
        "cmd": "ls"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with tty false
    // @step And yield_time_ms is None
    // @step And max_output_tokens is None
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: Value::String("ls".to_string()),
            workdir: None,
            tty: false,
            yield_time_ms: None,
            max_output_tokens: None,
            timeout_secs: None,
        }
    );
}

/// Scenario: CodexExecCommandFacade maps all optional params
#[test]
fn test_codex_exec_command_facade_all_params() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the Codex model calls exec_command with cmd "python3" workdir "/app" tty true yield_time_ms 10000 and max_output_tokens 4096
    let input = json!({
        "cmd": "python3",
        "workdir": "/app",
        "tty": true,
        "yield_time_ms": 10000,
        "max_output_tokens": 4096
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with command "python3"
    // @step And workdir is "/app"
    // @step And tty is true
    // @step And yield_time_ms is 10000
    // @step And max_output_tokens is 4096
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: Value::String("python3".to_string()),
            workdir: Some("/app".to_string()),
            tty: true,
            yield_time_ms: Some(10000),
            max_output_tokens: Some(4096),
            timeout_secs: None,
        }
    );
}

/// Scenario: CodexExecCommandFacade validates required cmd parameter
#[test]
fn test_codex_exec_command_facade_missing_cmd() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the Codex model calls exec_command with missing cmd field
    let input = json!({});
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error for tool "exec_command" mentioning "cmd"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "exec_command");
        assert!(message.contains("cmd"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexExecCommandFacade silently ignores Codex-native approval params
#[test]
fn test_codex_exec_command_facade_ignores_approval_params() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the Codex model calls exec_command with cmd "ls" and login true and shell "/bin/bash"
    let input = json!({
        "cmd": "ls",
        "login": true,
        "shell": "/bin/bash"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with command "ls"
    // @step And tty is false
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: Value::String("ls".to_string()),
            workdir: None,
            tty: false,
            yield_time_ms: None,
            max_output_tokens: None,
            timeout_secs: None,
        }
    );
}

/// Scenario: CodexExecCommandFacade schema has additionalProperties false
#[test]
fn test_codex_exec_command_facade_schema() {
    // @step Given a CodexExecCommandFacade instance
    let facade = CodexExecCommandFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has additionalProperties set to false
    assert_eq!(def.parameters["additionalProperties"], false);

    // @step And the required array contains only "cmd"
    assert_eq!(def.parameters["required"], json!(["cmd"]));

    // @step And the schema has properties for cmd workdir shell tty yield_time_ms max_output_tokens and login
    assert!(def.parameters["properties"]["cmd"].is_object());
    assert!(def.parameters["properties"]["workdir"].is_object());
    assert!(def.parameters["properties"]["shell"].is_object());
    assert!(def.parameters["properties"]["tty"].is_object());
    assert!(def.parameters["properties"]["yield_time_ms"].is_object());
    assert!(def.parameters["properties"]["max_output_tokens"].is_object());
    assert!(def.parameters["properties"]["login"].is_object());
}

/// Scenario: CodexExecCommandFacade rejects empty cmd
#[test]
fn test_codex_exec_command_facade_empty_cmd() {
    let facade = CodexExecCommandFacade;
    let input = json!({
        "cmd": ""
    });
    let result = facade.map_params(input);
    assert!(result.is_err());
}

/// Feature: spec/features/codex-shell-exec-facades.feature
///
/// Scenario: Both facades are registered in Codex create_rig_agent
#[test]
fn test_both_facades_tool_names_and_schemas() {
    // @step Given a CodexShellFacade and CodexExecCommandFacade instance
    let shell_facade = CodexShellFacade;
    let exec_facade = CodexExecCommandFacade;

    // @step When the Codex tool name list is inspected
    let shell_def = shell_facade.definition();
    let exec_def = exec_facade.definition();

    // @step Then "shell" is present and maps command as array type
    assert_eq!(shell_def.name, "shell");
    assert_eq!(
        shell_def.parameters["properties"]["command"]["type"],
        "array"
    );

    // @step And "exec_command" is present and maps cmd as string type
    assert_eq!(exec_def.name, "exec_command");
    assert_eq!(exec_def.parameters["properties"]["cmd"]["type"], "string");
}
