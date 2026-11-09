//! Feature: spec/features/codex-native-tool-facades.feature
//!
//! BUG-107/TOOL-015 shell_command, read_file and list_dir facade tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

// =========================================================================
// shell_command tests
// =========================================================================

/// Feature: spec/features/codex-native-tool-facades.feature
///
/// Scenario: CodexShellCommandFacade maps shell_command to InternalBashParams
#[test]
fn test_codex_shell_command_facade() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with command "ls -la" and workdir "/tmp"
    let input = json!({
        "command": "ls -la",
        "workdir": "/tmp"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalBashParams::Execute with command "ls -la"
    assert_eq!(
        result,
        InternalBashParams::Execute {
            command: "ls -la".to_string(),
            cwd: Some("/tmp".to_string()),
            timeout_ms: None,
        }
    );

    // @step And the facade tool name is "shell_command"
    assert_eq!(facade.tool_name(), "shell_command");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: Codex facades validate required parameters (shell_command variant)
#[test]
fn test_codex_shell_command_missing_command() {
    // @step Given a CodexShellCommandFacade instance
    let facade = CodexShellCommandFacade;

    // @step When the Codex model calls shell_command with missing command field
    let input = json!({});
    let result = facade.map_params(input);

    // @step Then the facade returns a validation error
    assert!(result.is_err());

    // @step And the error identifies "shell_command" as the tool
    // @step And the error mentions "command" as the missing field
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "shell_command");
        assert!(message.contains("command"));
    } else {
        panic!("Expected ToolError::Validation");
    }
}

#[test]
fn test_codex_shell_command_empty_command() {
    let facade = CodexShellCommandFacade;
    let input = json!({
        "command": ""
    });

    let result = facade.map_params(input);
    assert!(result.is_err());
}

// =========================================================================
// read_file tests
// =========================================================================

/// Scenario: CodexReadFileFacade maps read_file to InternalFileParams::Read
#[test]
fn test_codex_read_file_facade_with_offset_limit() {
    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with file_path "/src/main.rs" offset 10 and limit 50
    let input = json!({
        "file_path": "/src/main.rs",
        "offset": 10,
        "limit": 50
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs" offset 10 and limit 50
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: Some(10),
            limit: Some(50),
            mode: None,
            indentation: None,
        }
    );

    // @step And the facade tool name is "read_file"
    assert_eq!(facade.tool_name(), "read_file");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

/// Scenario: Codex facades handle optional parameters gracefully
#[test]
fn test_codex_read_file_facade_without_offset_limit() {
    // @step Given a CodexReadFileFacade instance
    let facade = CodexReadFileFacade;

    // @step When the Codex model calls read_file with only file_path "/src/main.rs"
    let input = json!({
        "file_path": "/src/main.rs"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalFileParams::Read with offset None and limit None
    assert_eq!(
        result,
        InternalFileParams::Read {
            file_path: "/src/main.rs".to_string(),
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        }
    );
}

#[test]
fn test_codex_read_file_missing_path() {
    let facade = CodexReadFileFacade;
    let input = json!({});

    let result = facade.map_params(input);
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "read_file");
        assert!(message.contains("file_path"));
    }
}

#[test]
fn test_codex_read_file_empty_path() {
    let facade = CodexReadFileFacade;
    let input = json!({
        "file_path": ""
    });

    let result = facade.map_params(input);
    assert!(result.is_err());
}

// =========================================================================
// list_dir tests
// =========================================================================

/// Scenario: CodexListDirFacade maps list_dir to InternalLsParams::List
#[test]
fn test_codex_list_dir_facade_with_dir_path() {
    // @step Given a CodexListDirFacade instance
    let facade = CodexListDirFacade;

    // @step When the Codex model calls list_dir with dir_path "/src"
    let input = json!({
        "dir_path": "/src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalLsParams::List with path "/src"
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: None,
            limit: None,
            depth: None,
        }
    );

    // @step And the facade tool name is "list_dir"
    assert_eq!(facade.tool_name(), "list_dir");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

#[test]
fn test_codex_list_dir_facade_with_empty_dir_path() {
    let facade = CodexListDirFacade;
    let input = json!({
        "dir_path": ""
    });

    let result = facade.map_params(input).unwrap();
    // Empty dir_path is treated as None (use default)
    assert_eq!(
        result,
        InternalLsParams::List {
            path: None,
            offset: None,
            limit: None,
            depth: None
        }
    );
}

#[test]
fn test_codex_list_dir_facade_with_null_dir_path() {
    let facade = CodexListDirFacade;
    let input = json!({
        "dir_path": null
    });

    let result = facade.map_params(input).unwrap();
    assert_eq!(
        result,
        InternalLsParams::List {
            path: None,
            offset: None,
            limit: None,
            depth: None
        }
    );
}
