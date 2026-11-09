//! Feature: spec/features/codex-shell-command-params.feature
//!
//! BUG-108 tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;
use crate::facade::bash::GeminiRunShellCommandFacade;

// =========================================================================
// BUG-108: shell_command workdir and timeout_ms mapping tests
// Feature: spec/features/codex-shell-command-params.feature
// =========================================================================

/// Scenario: CodexShellCommandFacade maps workdir to InternalBashParams cwd
#[test]
fn test_codex_shell_command_maps_workdir_to_cwd() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with command "make test" and workdir "/project"
    let input = json!({
        "command": "make test",
        "workdir": "/project"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "make test" and cwd "/project"
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "make test".to_string(),
            cwd: Some("/project".to_string()),
            timeout_ms: None,
        }
    );

    // @step And timeout_ms is None
    let InternalBashParams::Execute { timeout_ms, .. } = &result;
    assert_eq!(*timeout_ms, None);
}

/// Scenario: CodexShellCommandFacade maps timeout_ms to InternalBashParams
#[test]
fn test_codex_shell_command_maps_timeout_ms() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with command "sleep 100" and timeout_ms 5000
    let input = json!({
        "command": "sleep 100",
        "timeout_ms": 5000
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "sleep 100" and timeout_ms 5000
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "sleep 100".to_string(),
            cwd: None,
            timeout_ms: Some(5000),
        }
    );

    // @step And cwd is None
    let InternalBashParams::Execute { cwd, .. } = &result;
    assert_eq!(*cwd, None);
}

/// Scenario: CodexShellCommandFacade maps both workdir and timeout_ms
#[test]
fn test_codex_shell_command_maps_both_workdir_and_timeout() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with command "npm test" workdir "/app" and timeout_ms 30000
    let input = json!({
        "command": "npm test",
        "workdir": "/app",
        "timeout_ms": 30000
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "npm test" cwd "/app" and timeout_ms 30000
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "npm test".to_string(),
            cwd: Some("/app".to_string()),
            timeout_ms: Some(30000),
        }
    );
}

/// Scenario: CodexShellCommandFacade without optional params defaults to None
#[test]
fn test_codex_shell_command_without_optional_params() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with only command "echo hello"
    let input = json!({
        "command": "echo hello"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "echo hello"
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "echo hello".to_string(),
            cwd: None,
            timeout_ms: None,
        }
    );

    // @step And cwd is None
    // @step And timeout_ms is None
    let InternalBashParams::Execute {
        cwd, timeout_ms, ..
    } = &result;
    assert_eq!(*cwd, None);
    assert_eq!(*timeout_ms, None);
}

/// Scenario: Codex shell_command schema includes Codex-native approval params
#[test]
fn test_codex_shell_command_schema_has_approval_params() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the tool definition schema is inspected
    let def = facade.definition();

    // @step Then the schema has a "login" property of type "boolean"
    assert_eq!(def.parameters["properties"]["login"]["type"], "boolean");

    // @step And the schema has a "sandbox_permissions" property of type "string"
    assert_eq!(
        def.parameters["properties"]["sandbox_permissions"]["type"],
        "string"
    );

    // @step And the schema has a "justification" property of type "string"
    assert_eq!(
        def.parameters["properties"]["justification"]["type"],
        "string"
    );

    // @step And the schema has a "prefix_rule" property of type "array"
    assert_eq!(def.parameters["properties"]["prefix_rule"]["type"], "array");
}

/// Scenario: Codex-native approval params are silently ignored in map_params
#[test]
fn test_codex_shell_command_approval_params_ignored() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with command "ls" and login true and sandbox_permissions "use_default"
    let input = json!({
        "command": "ls",
        "login": true,
        "sandbox_permissions": "use_default",
        "justification": "testing",
        "prefix_rule": ["ls"]
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "ls"
    // @step And cwd is None
    // @step And timeout_ms is None
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "ls".to_string(),
            cwd: None,
            timeout_ms: None,
        }
    );
}

/// Scenario: Existing facades remain backward compatible with new InternalBashParams fields
#[test]
fn test_gemini_facade_backward_compatible_with_new_fields() {
    // @step Given a GeminiRunShellCommandFacade instance
    let facade = GeminiRunShellCommandFacade;

    // @step When the Gemini model calls run_shell_command with command "ls"
    let input = json!({
        "command": "ls"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "ls"
    // @step And cwd is None
    // @step And timeout_ms is None
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "ls".to_string(),
            cwd: None,
            timeout_ms: None,
        }
    );
}
