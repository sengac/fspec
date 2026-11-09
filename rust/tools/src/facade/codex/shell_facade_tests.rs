//! Feature: spec/features/codex-shell-exec-facades.feature
//!
//! BUG-114 shell facade tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// BUG-114: shell facade tests (ExecToolFacade)
// Feature: spec/features/codex-shell-exec-facades.feature
// =========================================================================

/// Scenario: CodexShellFacade maps shell command array to InternalExecParams::Run
#[test]
fn test_codex_shell_facade_maps_command_array() {
    // @step Given a CodexShellFacade instance
    let facade = CodexShellFacade;

    // @step When the Codex model calls shell with command ["ls", "-la"] and workdir "/tmp"
    let input = json!({
        "command": ["ls", "-la"],
        "workdir": "/tmp"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with command as JSON array ["ls", "-la"]
    // @step And tty is false
    // @step And workdir is "/tmp"
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: json!(["ls", "-la"]),
            workdir: Some("/tmp".to_string()),
            tty: false,
            yield_time_ms: None,
            max_output_tokens: None,
            timeout_secs: None,
        }
    );

    // @step And the facade tool name is "shell"
    assert_eq!(facade.tool_name(), "shell");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: CodexShellFacade converts timeout_ms to timeout_secs
#[test]
fn test_codex_shell_facade_converts_timeout() {
    // @step Given a CodexShellFacade instance
    let facade = CodexShellFacade;

    // @step When the Codex model calls shell with command ["git", "status"] and timeout_ms 5000
    let input = json!({
        "command": ["git", "status"],
        "timeout_ms": 5000
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with timeout_secs 5
    if let InternalExecParams::Run { timeout_secs, .. } = &result {
        assert_eq!(*timeout_secs, Some(5));
    } else {
        panic!("Expected InternalExecParams::Run");
    }
}

/// Scenario: CodexShellFacade without optional params defaults to None
#[test]
fn test_codex_shell_facade_defaults_to_none() {
    // @step Given a CodexShellFacade instance
    let facade = CodexShellFacade;

    // @step When the Codex model calls shell with only command ["echo", "hello"]
    let input = json!({
        "command": ["echo", "hello"]
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalExecParams::Run with workdir None
    // @step And timeout_secs is None
    // @step And yield_time_ms is None
    assert_eq!(
        result,
        InternalExecParams::Run {
            command: json!(["echo", "hello"]),
            workdir: None,
            tty: false,
            yield_time_ms: None,
            max_output_tokens: None,
            timeout_secs: None,
        }
    );
}

/// Scenario: CodexShellFacade validates required command parameter
#[test]
fn test_codex_shell_facade_missing_command() {
    // @step Given a CodexShellFacade instance
    let facade = CodexShellFacade;

    // @step When the Codex model calls shell with missing command field
    let input = json!({});
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error for tool "shell" mentioning "command"
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "shell");
        assert!(message.contains("command"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

/// Scenario: CodexShellFacade rejects empty command array
#[test]
fn test_codex_shell_facade_empty_command_array() {
    let facade = CodexShellFacade;
    let input = json!({
        "command": []
    });
    let result = facade.map_params(input);
    assert!(result.is_err());
}

/// Scenario: CodexShellFacade rejects non-array command
#[test]
fn test_codex_shell_facade_non_array_command() {
    let facade = CodexShellFacade;
    let input = json!({
        "command": "ls -la"
    });
    let result = facade.map_params(input);
    assert!(result.is_err());
}

/// Scenario: CodexShellFacade schema has additionalProperties false
#[test]
fn test_codex_shell_facade_schema() {
    // @step Given a CodexShellFacade instance
    let facade = CodexShellFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has additionalProperties set to false
    assert_eq!(def.parameters["additionalProperties"], false);

    // @step And the required array contains only "command"
    assert_eq!(def.parameters["required"], json!(["command"]));

    // @step And command property type is "array" with items type "string"
    assert_eq!(def.parameters["properties"]["command"]["type"], "array");
    assert_eq!(
        def.parameters["properties"]["command"]["items"]["type"],
        "string"
    );
}
