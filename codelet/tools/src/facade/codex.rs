//! Codex-specific tool facades.
//!
//! These facades adapt the internal tool interfaces for Codex (GPT-5.1-codex) models,
//! using the exact tool names and parameter schemas from the Codex CLI
//! (codex-rs/core/src/tools/spec.rs).
//!
//! Codex models are trained on:
//! - `shell_command` (not `Bash` or `run_command`)
//! - `read_file` with `file_path` parameter
//! - `list_dir` with `dir_path` parameter (not `path`)
//! - `grep_files` with `pattern`, `include`, `path`, `limit` parameters
//!
//! Feature: spec/features/codex-native-tool-facades.feature

use super::traits::{
    BashToolFacade, FileToolFacade, InternalBashParams, InternalFileParams, InternalLsParams,
    InternalSearchParams, LsToolFacade, SearchToolFacade, ToolDefinition,
};
use crate::ToolError;
use serde_json::{json, Value};

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract a required non-empty string field from JSON input.
/// Returns an error if the field is missing, null, or empty.
fn extract_required_string(input: &Value, field: &str, tool: &'static str) -> Result<String, ToolError> {
    let value = input
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| ToolError::Validation {
            tool,
            message: format!("Missing or empty required '{field}' field"),
        })?;
    Ok(value.to_string())
}

/// Extract an optional string field from JSON input.
/// Returns None if the field is missing, null, or empty.
fn extract_optional_string(input: &Value, field: &str) -> Option<String> {
    input
        .get(field)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// Extract an optional unsigned integer field from JSON input.
fn extract_optional_uint(input: &Value, field: &str) -> Option<usize> {
    input.get(field).and_then(Value::as_u64).map(|n| n as usize)
}

// ============================================================================
// Shell Command Facade
// ============================================================================

/// Codex-specific facade for shell command execution.
///
/// Maps Codex's `shell_command` tool to the internal BashTool parameters.
/// The Codex CLI defines `shell_command` with:
/// - `command` (required): The shell script to execute
/// - `workdir` (optional): Working directory for execution
/// - `timeout_ms` (optional): Timeout in milliseconds
///
/// Note: `workdir` is exposed in the schema for model compatibility but
/// the actual CWD override is handled by the BashToolFacadeWrapper via
/// the session isolation context (GIT-020), not by this facade.
pub struct CodexShellCommandFacade;

impl BashToolFacade for CodexShellCommandFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "shell_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "shell_command".to_string(),
            description: "Execute a shell command in the user's default shell. Returns stdout on success, stderr on failure.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell script to execute in the user's default shell"
                    },
                    "workdir": {
                        "type": "string",
                        "description": "The working directory to execute the command in"
                    },
                    "timeout_ms": {
                        "type": "integer",
                        "description": "The timeout for the command in milliseconds"
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
        let command = extract_required_string(&input, "command", "shell_command")?;
        Ok(InternalBashParams::Execute { command })
    }
}

// ============================================================================
// Read File Facade
// ============================================================================

/// Codex-specific facade for file reading.
///
/// Maps Codex's `read_file` tool to the internal ReadTool parameters.
/// The Codex CLI defines `read_file` with:
/// - `file_path` (required): Absolute path to the file
/// - `offset` (optional): Line number to start reading from (1-based)
/// - `limit` (optional): Maximum number of lines to return
pub struct CodexReadFileFacade;

impl FileToolFacade for CodexReadFileFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "read_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read file contents. Supports text files, images (PNG, JPG, GIF, WEBP, SVG), and PDFs. Use offset/limit for large files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to read"
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The line number to start reading from. Must be 1 or greater."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The maximum number of lines to return."
                    }
                },
                "required": ["file_path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = extract_required_string(&input, "file_path", "read_file")?;
        let offset = extract_optional_uint(&input, "offset");
        let limit = extract_optional_uint(&input, "limit");

        Ok(InternalFileParams::Read {
            file_path,
            offset,
            limit,
        })
    }
}

// ============================================================================
// List Directory Facade
// ============================================================================

/// Codex-specific facade for directory listing.
///
/// Maps Codex's `list_dir` tool to the internal LsTool parameters.
/// The Codex CLI defines `list_dir` with:
/// - `dir_path` (required): Absolute path to the directory to list
/// - `offset` (optional): Entry number to start listing from
/// - `limit` (optional): Maximum number of entries to return
/// - `depth` (optional): Maximum directory depth to traverse
///
/// Note: Unlike Z.AI's `list_dir` which uses `path`, Codex uses `dir_path`.
pub struct CodexListDirFacade;

impl LsToolFacade for CodexListDirFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "list_dir"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".to_string(),
            description: "List directory contents with file metadata. Returns formatted output showing permissions, size, modification time, and name for each entry.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "dir_path": {
                        "type": "string",
                        "description": "Absolute path to the directory to list."
                    },
                    "depth": {
                        "type": "integer",
                        "description": "The maximum directory depth to traverse. Must be 1 or greater."
                    }
                },
                "required": ["dir_path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError> {
        // Codex uses `dir_path`, not `path`
        let dir_path = extract_optional_string(&input, "dir_path");
        Ok(InternalLsParams::List { path: dir_path })
    }
}

// ============================================================================
// Grep Files Facade
// ============================================================================

/// Codex-specific facade for file content searching.
///
/// Maps Codex's `grep_files` tool to the internal GrepTool parameters.
/// The Codex CLI defines `grep_files` with:
/// - `pattern` (required): Regular expression pattern to search for
/// - `include` (optional): Glob that limits which files are searched (e.g., '*.rs')
/// - `path` (optional): Directory or file path to search
/// - `limit` (optional): Maximum number of file paths to return
pub struct CodexGrepFilesFacade;

impl SearchToolFacade for CodexGrepFilesFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "grep_files"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep_files".to_string(),
            description: "Search file contents using regex pattern. Returns matching lines with file paths and line numbers. Supports glob filters to limit which files are searched.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regular expression pattern to search for."
                    },
                    "include": {
                        "type": "string",
                        "description": "Optional glob that limits which files are searched (e.g. '*.rs')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory or file path to search. Defaults to session working directory."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of file paths to return (defaults to 100)."
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = extract_required_string(&input, "pattern", "grep_files")?;
        let path = extract_optional_string(&input, "path");

        Ok(InternalSearchParams::Grep { pattern, path })
    }
}

// ============================================================================
// Glob Facade
// ============================================================================

/// Codex-specific facade for file pattern matching.
///
/// Exposes the glob tool under lowercase `glob`, which Codex expects.
pub struct CodexGlobFacade;

impl SearchToolFacade for CodexGlobFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "glob"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".to_string(),
            description: "Fast file pattern matching tool that works with any codebase size. Supports glob patterns like \"**/*.js\" or \"src/**/*.ts\". Returns matching file paths one per line.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (optional, defaults to current directory)"
                    },
                    "case_insensitive": {
                        "type": "boolean",
                        "description": "Whether to perform case-insensitive matching (optional, defaults to false)"
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = extract_required_string(&input, "pattern", "glob")?;
        let path = extract_optional_string(&input, "path");

        Ok(InternalSearchParams::Glob { pattern, path })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
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
                command: "ls -la".to_string()
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
                path: Some("/src".to_string())
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
        assert_eq!(result, InternalLsParams::List { path: None });
    }

    #[test]
    fn test_codex_list_dir_facade_with_null_dir_path() {
        let facade = CodexListDirFacade;
        let input = json!({
            "dir_path": null
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None });
    }

    #[test]
    fn test_codex_list_dir_uses_dir_path_not_path() {
        // Verify that `dir_path` is used, not `path`
        let facade = CodexListDirFacade;

        // If model sends `path` (wrong field name), it should NOT be picked up
        let input = json!({
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None });

        // But `dir_path` should work
        let input = json!({
            "dir_path": "/src"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string())
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
                path: Some("/src".to_string())
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
                path: None
            }
        );
    }

    #[test]
    fn test_codex_grep_files_facade_with_include_filter() {
        let facade = CodexGrepFilesFacade;
        let input = json!({
            "pattern": "TODO",
            "include": "*.rs",
            "path": "/src"
        });

        // `include` is accepted in the schema but not mapped to InternalSearchParams
        // (our internal grep handles file filtering separately)
        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("/src".to_string())
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

    #[test]
    fn test_codex_glob_facade() {
        let facade = CodexGlobFacade;
        let input = json!({
            "pattern": "**/*.rs",
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        assert_eq!(
            result,
            InternalSearchParams::Glob {
                pattern: "**/*.rs".to_string(),
                path: Some("/src".to_string())
            }
        );

        assert_eq!(facade.tool_name(), "glob");
        assert_eq!(facade.provider(), "codex");
    }

    #[test]
    fn test_codex_glob_facade_no_path() {
        let facade = CodexGlobFacade;
        let input = json!({
            "pattern": "**/*.rs"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalSearchParams::Glob {
                pattern: "**/*.rs".to_string(),
                path: None
            }
        );
    }

    #[test]
    fn test_codex_glob_facade_missing_pattern() {
        let facade = CodexGlobFacade;
        let input = json!({
            "path": "/src"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "glob");
            assert!(message.contains("pattern"));
        }
    }

    // =========================================================================
    // Tool naming tests
    // =========================================================================

    #[test]
    fn test_codex_tools_use_correct_names() {
        assert_eq!(CodexShellCommandFacade.tool_name(), "shell_command");
        assert_eq!(CodexReadFileFacade.tool_name(), "read_file");
        assert_eq!(CodexListDirFacade.tool_name(), "list_dir");
        assert_eq!(CodexGrepFilesFacade.tool_name(), "grep_files");
        assert_eq!(CodexGlobFacade.tool_name(), "glob");
    }

    #[test]
    fn test_codex_tools_provider_name() {
        assert_eq!(CodexShellCommandFacade.provider(), "codex");
        assert_eq!(CodexReadFileFacade.provider(), "codex");
        assert_eq!(CodexListDirFacade.provider(), "codex");
        assert_eq!(CodexGrepFilesFacade.provider(), "codex");
        assert_eq!(CodexGlobFacade.provider(), "codex");
    }

    // =========================================================================
    // Schema validation tests
    // =========================================================================

    /// Scenario: Codex tool schemas use additionalProperties false
    #[test]
    fn test_all_codex_schemas_have_additional_properties_false() {
        // @step Given all Codex facade instances
        let facades: Vec<(&str, serde_json::Value)> = vec![
            ("shell_command", CodexShellCommandFacade.definition().parameters),
            ("read_file", CodexReadFileFacade.definition().parameters),
            ("list_dir", CodexListDirFacade.definition().parameters),
            ("grep_files", CodexGrepFilesFacade.definition().parameters),
            ("glob", CodexGlobFacade.definition().parameters),
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

        let glob_params = &facades[4].1;
        assert_eq!(glob_params["required"], json!(["pattern"]));
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
    fn test_codex_glob_schema_has_case_insensitive_param() {
        let facade = CodexGlobFacade;
        let def = facade.definition();

        assert!(def.parameters["properties"]["case_insensitive"].is_object());
        assert!(def.parameters["properties"]["pattern"].is_object());
    }

    #[test]
    fn test_codex_shell_command_schema_has_workdir_and_timeout() {
        let facade = CodexShellCommandFacade;
        let def = facade.definition();

        // Verify schema has Codex-specific optional params
        assert!(def.parameters["properties"]["workdir"].is_object());
        assert!(def.parameters["properties"]["timeout_ms"].is_object());
    }
}
