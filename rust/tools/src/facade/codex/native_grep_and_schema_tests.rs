//! Feature: spec/features/codex-native-tool-facades.feature
//!
//! BUG-107/TOOL-015 grep_files facade, tool naming and schema tests, moved out of codex.rs (COV-075: one test file per feature).

use super::*;

#[test]
fn test_codex_list_dir_uses_dir_path_not_path() {
    // Verify that `dir_path` is used, not `path`
    let facade = CodexListDirFacade;

    // If model sends `path` (wrong field name), it should NOT be picked up
    let input = json!({
        "path": "/src"
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

    // But `dir_path` should work
    let input = json!({
        "dir_path": "/src"
    });
    let result = facade.map_params(input).unwrap();
    assert_eq!(
        result,
        InternalLsParams::List {
            path: Some("/src".to_string()),
            offset: None,
            limit: None,
            depth: None,
        }
    );
}

// =========================================================================
// grep_files tests
// =========================================================================

/// Scenario: CodexGrepFilesFacade maps grep_files to InternalSearchParams::Grep
#[test]
fn test_codex_grep_files_facade() {
    // @step Given a CodexGrepFilesFacade instance
    let facade = CodexGrepFilesFacade;

    // @step When the Codex model calls grep_files with pattern "TODO" and path "/src"
    let input = json!({
        "pattern": "TODO",
        "path": "/src"
    });
    let result = facade.map_params(input).unwrap();

    // @step Then the facade maps to InternalSearchParams::Grep with pattern "TODO" and path "/src"
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: Some("/src".to_string()),
            include: None,
            limit: None,
        }
    );

    // @step And the facade tool name is "grep_files"
    assert_eq!(facade.tool_name(), "grep_files");

    // @step And the facade provider is "codex"
    assert_eq!(facade.provider(), "codex");
}

#[test]
fn test_codex_grep_files_facade_no_path() {
    let facade = CodexGrepFilesFacade;
    let input = json!({
        "pattern": "TODO"
    });

    let result = facade.map_params(input).unwrap();
    assert_eq!(
        result,
        InternalSearchParams::Grep {
            pattern: "TODO".to_string(),
            path: None,
            include: None,
            limit: None,
        }
    );
}

#[test]
fn test_codex_grep_files_facade_missing_pattern() {
    let facade = CodexGrepFilesFacade;
    let input = json!({
        "path": "/src"
    });

    let result = facade.map_params(input);
    assert!(result.is_err());
    if let Err(ToolError::Validation { tool, message }) = result {
        assert_eq!(tool, "grep_files");
        assert!(message.contains("pattern"));
    }
}

// =========================================================================
// Tool naming tests
// =========================================================================

/// BUG-107: Verify only native Codex CLI tools are exposed (no glob)
///
/// Feature: spec/features/codex-native-tool-facades.feature
///
/// Scenario: Codex agent does not expose non-native glob tool
#[test]
fn test_codex_does_not_expose_non_native_glob() {
    // @step Given a Codex agent built with create_rig_agent
    // Collect all facade tool names that would be registered
    let codex_facade_names = [
        CodexShellCommandFacade.tool_name(),
        CodexReadFileFacade.tool_name(),
        CodexListDirFacade.tool_name(),
        CodexGrepFilesFacade.tool_name(),
        CodexViewImageFacade.tool_name(),
    ];

    // @step When the agent tool definitions are inspected
    // @step Then the tool list does not contain "glob"
    assert!(
        !codex_facade_names.contains(&"glob"),
        "Codex facades should NOT include 'glob' - it is not a native Codex CLI tool"
    );

    // @step And the tool list contains "shell_command"
    assert!(codex_facade_names.contains(&"shell_command"));
    // @step And the tool list contains "read_file"
    assert!(codex_facade_names.contains(&"read_file"));
    // @step And the tool list contains "list_dir"
    assert!(codex_facade_names.contains(&"list_dir"));
    // @step And the tool list contains "grep_files"
    assert!(codex_facade_names.contains(&"grep_files"));
    // @step And the tool list contains "apply_patch"
    // Note: apply_patch is a standalone tool (not a facade), tested in codex provider tests
}

#[test]
fn test_codex_tools_use_correct_names() {
    assert_eq!(CodexShellCommandFacade.tool_name(), "shell_command");
    assert_eq!(CodexReadFileFacade.tool_name(), "read_file");
    assert_eq!(CodexListDirFacade.tool_name(), "list_dir");
    assert_eq!(CodexGrepFilesFacade.tool_name(), "grep_files");
    assert_eq!(CodexViewImageFacade.tool_name(), "view_image");
}

#[test]
fn test_codex_tools_provider_name() {
    assert_eq!(CodexShellCommandFacade.provider(), "codex");
    assert_eq!(CodexReadFileFacade.provider(), "codex");
    assert_eq!(CodexListDirFacade.provider(), "codex");
    assert_eq!(CodexGrepFilesFacade.provider(), "codex");
    assert_eq!(CodexViewImageFacade.provider(), "codex");
}

// =========================================================================
// Schema validation tests
// =========================================================================

/// Scenario: Codex tool schemas use additionalProperties false
#[test]
fn test_all_codex_schemas_have_additional_properties_false() {
    // @step Given all Codex facade instances
    let facades: Vec<(&str, serde_json::Value)> = vec![
        (
            "shell_command",
            CodexShellCommandFacade.definition().parameters,
        ),
        ("read_file", CodexReadFileFacade.definition().parameters),
        ("list_dir", CodexListDirFacade.definition().parameters),
        ("grep_files", CodexGrepFilesFacade.definition().parameters),
        ("view_image", CodexViewImageFacade.definition().parameters),
    ];

    // @step When their tool definitions are inspected
    // @step Then each schema has additionalProperties set to false
    for (name, params) in &facades {
        assert_eq!(
            params["additionalProperties"], false,
            "{name} should have additionalProperties: false"
        );
    }

    // @step And each schema has the correct required fields
    let shell_params = &facades[0].1;
    assert_eq!(shell_params["required"], json!(["command"]));

    let read_params = &facades[1].1;
    assert_eq!(read_params["required"], json!(["file_path"]));

    let list_params = &facades[2].1;
    assert_eq!(list_params["required"], json!(["dir_path"]));

    let grep_params = &facades[3].1;
    assert_eq!(grep_params["required"], json!(["pattern"]));
}

#[test]
fn test_codex_list_dir_schema_uses_dir_path_not_path() {
    let facade = CodexListDirFacade;
    let def = facade.definition();

    // Verify schema uses dir_path, not path
    assert!(def.parameters["properties"]["dir_path"].is_object());
    assert!(def.parameters["properties"].get("path").is_none());
}

#[test]
fn test_codex_grep_files_schema_has_include_param() {
    let facade = CodexGrepFilesFacade;
    let def = facade.definition();

    // Verify schema has `include` parameter (Codex-specific)
    assert!(def.parameters["properties"]["include"].is_object());
}

#[test]
fn test_codex_shell_command_schema_has_workdir_and_timeout() {
    let facade = CodexShellCommandFacade;
    let def = facade.definition();

    // Verify schema has Codex-specific optional params
    assert!(def.parameters["properties"]["workdir"].is_object());
    assert!(def.parameters["properties"]["timeout_ms"].is_object());
}
