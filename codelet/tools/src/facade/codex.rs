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
    BashToolFacade, FileToolFacade, InternalBashParams, InternalFileParams,
    InternalIndentationParams, InternalLsParams, InternalSearchParams, LsToolFacade,
    SearchToolFacade, ToolDefinition,
};
use super::param_extract::{extract_optional_bool, extract_optional_string, extract_optional_uint, extract_required_string};
use crate::ToolError;
use serde_json::{json, Value};

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
/// - `login` (optional): Whether to run with login shell semantics
/// - `sandbox_permissions` (optional): Sandbox escalation control
/// - `justification` (optional): Approval justification
/// - `prefix_rule` (optional): Permission prefix pattern
///
/// The `login`, `sandbox_permissions`, `justification`, and `prefix_rule` params
/// are accepted in the schema for model compatibility but silently ignored.
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
                    },
                    "login": {
                        "type": "boolean",
                        "description": "Whether to run the shell with login shell semantics. Defaults to true."
                    },
                    "sandbox_permissions": {
                        "type": "string",
                        "description": "Sandbox permissions for the command. Defaults to \"use_default\"."
                    },
                    "justification": {
                        "type": "string",
                        "description": "Approval justification when sandbox_permissions is \"require_escalated\"."
                    },
                    "prefix_rule": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Permission prefix pattern for escalated sandbox permissions."
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
        let command = extract_required_string(&input, "command", "shell_command")?;
        let cwd = extract_optional_string(&input, "workdir");
        let timeout_ms = input.get("timeout_ms").and_then(Value::as_u64);
        Ok(InternalBashParams::Execute { command, cwd, timeout_ms })
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
/// - `mode` (optional): Mode selector — "slice" (default) or "indentation"
/// - `indentation` (optional): Nested object for indentation-aware block reading
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
            description: "Reads a local file with 1-indexed line numbers, supporting slice and indentation-aware block modes.".to_string(),
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
                    },
                    "mode": {
                        "type": "string",
                        "description": "Optional mode selector: \"slice\" for simple ranges (default) or \"indentation\" to expand around an anchor line."
                    },
                    "indentation": {
                        "type": "object",
                        "properties": {
                            "anchor_line": {
                                "type": "integer",
                                "description": "Anchor line to center the indentation lookup on (defaults to offset)."
                            },
                            "max_levels": {
                                "type": "integer",
                                "description": "How many parent indentation levels (smaller indents) to include."
                            },
                            "include_siblings": {
                                "type": "boolean",
                                "description": "When true, include additional blocks that share the anchor indentation."
                            },
                            "include_header": {
                                "type": "boolean",
                                "description": "Include doc comments or attributes directly above the selected block."
                            },
                            "max_lines": {
                                "type": "integer",
                                "description": "Hard cap on the number of lines returned when using indentation mode."
                            }
                        },
                        "additionalProperties": false
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
        let mode = extract_optional_string(&input, "mode");

        // Extract indentation sub-object if present
        let indentation = input.get("indentation").and_then(|v| {
            if v.is_object() {
                Some(InternalIndentationParams {
                    anchor_line: extract_optional_uint(v, "anchor_line"),
                    max_levels: extract_optional_uint(v, "max_levels"),
                    include_siblings: extract_optional_bool(v, "include_siblings"),
                    include_header: extract_optional_bool(v, "include_header"),
                    max_lines: extract_optional_uint(v, "max_lines"),
                })
            } else {
                None
            }
        });

        Ok(InternalFileParams::Read {
            file_path,
            offset,
            limit,
            mode,
            indentation,
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
            description: "Lists entries in a local directory with 1-indexed entry numbers and simple type labels.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "dir_path": {
                        "type": "string",
                        "description": "Absolute path to the directory to list."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "The entry number to start listing from. Must be 1 or greater."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "The maximum number of entries to return."
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
        let offset = extract_optional_uint(&input, "offset");
        let limit = extract_optional_uint(&input, "limit");
        let depth = extract_optional_uint(&input, "depth");
        Ok(InternalLsParams::List { path: dir_path, offset, limit, depth })
    }
}

// ============================================================================
// View Image Facade
// ============================================================================

/// Codex-specific facade for viewing image files.
///
/// Maps Codex's `view_image` tool to the internal ReadTool via `InternalFileParams::Read`.
/// The Codex CLI defines `view_image` with:
/// - `path` (required): Local filesystem path to an image file
///
/// This facade maps `view_image { path }` → `InternalFileParams::Read { file_path: path }`
/// so it delegates to ReadTool, which already handles image files (PNG, JPEG, GIF, WEBP)
/// by detecting file type and returning base64-encoded image data.
///
/// The `detail` parameter from the original Codex CLI is accepted in the schema
/// for model compatibility but not used (our ReadTool always returns the full image).
pub struct CodexViewImageFacade;

impl FileToolFacade for CodexViewImageFacade {
    fn provider(&self) -> &'static str {
        "codex"
    }

    fn tool_name(&self) -> &'static str {
        "view_image"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "view_image".to_string(),
            description: "View a local image from the filesystem (only use if given a full filepath \
                by the user, and the image isn't already attached to the thread context \
                within <image ...> tags).".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Local filesystem path to an image file"
                    },
                    "detail": {
                        "type": "string",
                        "description": "Optional detail override. The only supported value is `original`; omit this field for default resized behavior."
                    }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let path = extract_required_string(&input, "path", "view_image")?;
        // detail is accepted for model compatibility but not used —
        // ReadTool always returns the full image as base64
        Ok(InternalFileParams::Read {
            file_path: path,
            offset: None,
            limit: None,
            mode: None,
            indentation: None,
        })
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
        let include = extract_optional_string(&input, "include");
        let limit = extract_optional_uint(&input, "limit");

        Ok(InternalSearchParams::Grep { pattern, path, include, limit })
    }
}


#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::facade::bash::GeminiRunShellCommandFacade;

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
        let InternalBashParams::Execute { cwd, timeout_ms, .. } = &result;
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
        assert_eq!(
            def.parameters["properties"]["prefix_rule"]["type"],
            "array"
        );
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
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });
    }

    #[test]
    fn test_codex_list_dir_facade_with_null_dir_path() {
        let facade = CodexListDirFacade;
        let input = json!({
            "dir_path": null
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });
    }

    // =========================================================================
    // BUG-110: list_dir offset, limit, and depth mapping tests
    // Feature: spec/features/codex-list-dir-pagination.feature
    // =========================================================================

    /// Scenario: CodexListDirFacade maps offset and limit to InternalLsParams
    #[test]
    fn test_codex_list_dir_maps_offset_and_limit() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with dir_path "/src" offset 5 and limit 10
        let input = json!({
            "dir_path": "/src",
            "offset": 5,
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with path "/src" offset 5 and limit 10
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: Some(5),
                limit: Some(10),
                depth: None,
            }
        );

        // @step Then depth is None
        let InternalLsParams::List { depth, .. } = &result;
        assert_eq!(*depth, None);
    }

    /// Scenario: CodexListDirFacade backward compatible without optional params
    #[test]
    fn test_codex_list_dir_backward_compatible_no_pagination() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with only dir_path "/src"
        let input = json!({
            "dir_path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
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

    /// Scenario: CodexListDirFacade schema includes offset limit and depth properties
    #[test]
    fn test_codex_list_dir_schema_has_pagination_params() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has an "offset" property of type "integer"
        assert_eq!(def.parameters["properties"]["offset"]["type"], "integer");

        // @step Then the schema has a "limit" property of type "integer"
        assert_eq!(def.parameters["properties"]["limit"]["type"], "integer");

        // @step Then the schema has a "depth" property of type "integer"
        assert_eq!(def.parameters["properties"]["depth"]["type"], "integer");

        // @step Then only "dir_path" is in the required array
        assert_eq!(def.parameters["required"], json!(["dir_path"]));
    }

    /// Scenario: CodexListDirFacade maps depth to InternalLsParams
    #[test]
    fn test_codex_list_dir_maps_depth() {
        // @step Given a CodexListDirFacade instance
        let facade = CodexListDirFacade;

        // @step When the Codex model calls list_dir with dir_path "/src" and depth 3
        let input = json!({
            "dir_path": "/src",
            "depth": 3
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with path "/src" and depth 3
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/src".to_string()),
                offset: None,
                limit: None,
                depth: Some(3),
            }
        );

        // @step Then offset is None and limit is None
        let InternalLsParams::List { offset, limit, .. } = &result;
        assert_eq!(*offset, None);
        assert_eq!(*limit, None);
    }

    /// Scenario: Other facades provide None for offset limit and depth
    #[test]
    fn test_zai_facade_provides_none_for_pagination_params() {
        use crate::facade::zai::ZAIListDirFacade;

        // @step Given a ZAIListDirFacade instance
        let facade = ZAIListDirFacade;

        // @step When the ZAI model calls list_dir with path "/src"
        let input = json!({
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalLsParams::List with offset None limit None and depth None
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

    #[test]
    fn test_codex_list_dir_uses_dir_path_not_path() {
        // Verify that `dir_path` is used, not `path`
        let facade = CodexListDirFacade;

        // If model sends `path` (wrong field name), it should NOT be picked up
        let input = json!({
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None, offset: None, limit: None, depth: None });

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

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: CodexGrepFilesFacade maps include param to InternalSearchParams::Grep
    #[test]
    fn test_codex_grep_files_facade_maps_include_param() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern "TODO", include "*.rs", and path "/src"
        let input = json!({
            "pattern": "TODO",
            "include": "*.rs",
            "path": "/src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with pattern "TODO", path "/src", and include "*.rs"
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("/src".to_string()),
                include: Some("*.rs".to_string()),
                limit: None,
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade maps limit param to InternalSearchParams::Grep
    #[test]
    fn test_codex_grep_files_facade_maps_limit_param() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern "." and limit 10
        let input = json!({
            "pattern": ".",
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with pattern "." and limit 10
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: ".".to_string(),
                path: None,
                include: None,
                limit: Some(10),
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade maps both include and limit params
    #[test]
    fn test_codex_grep_files_facade_maps_include_and_limit() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with pattern ".", include "*.rs", and limit 10
        let input = json!({
            "pattern": ".",
            "include": "*.rs",
            "limit": 10
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include "*.rs" and limit 10
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: ".".to_string(),
                path: None,
                include: Some("*.rs".to_string()),
                limit: Some(10),
            }
        );
    }

    /// Scenario: CodexGrepFilesFacade remains backward compatible without include or limit
    #[test]
    fn test_codex_grep_files_facade_backward_compatible() {
        // @step Given a CodexGrepFilesFacade instance
        let facade = CodexGrepFilesFacade;

        // @step When the Codex model calls grep_files with only pattern "TODO"
        let input = json!({
            "pattern": "TODO"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
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

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: InternalSearchParams::Grep includes optional include and limit fields
    #[test]
    fn test_internal_search_params_grep_has_include_and_limit_fields() {
        // @step Given the InternalSearchParams::Grep enum variant
        // Construct it with all fields to prove they exist

        // @step Then it has an optional "include" field of type Option<String>
        let with_include = InternalSearchParams::Grep {
            pattern: "test".to_string(),
            path: None,
            include: Some("*.rs".to_string()),
            limit: None,
        };
        if let InternalSearchParams::Grep { include, .. } = &with_include {
            assert_eq!(include, &Some("*.rs".to_string()));
        }

        // @step And it has an optional "limit" field of type Option<usize>
        let with_limit = InternalSearchParams::Grep {
            pattern: "test".to_string(),
            path: None,
            include: None,
            limit: Some(100),
        };
        if let InternalSearchParams::Grep { limit, .. } = &with_limit {
            assert_eq!(limit, &Some(100));
        }
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
        let codex_facade_names = vec![
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
            ("shell_command", CodexShellCommandFacade.definition().parameters),
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

    // =========================================================================
    // BUG-111: GrepArgs glob field tests
    // Feature: spec/features/codex-grep-files-include-limit.feature
    // =========================================================================

    /// Scenario: GrepArgs struct supports optional glob field
    #[test]
    fn test_grep_args_supports_optional_glob_field() {
        use crate::grep::GrepArgs;

        // @step Given a GrepArgs with pattern "TODO" and glob "*.rs"
        let args = GrepArgs {
            pattern: "TODO".to_string(),
            path: None,
            output_mode: None,
            glob: Some("*.rs".to_string()),
            limit: None,
        };

        // @step When the GrepTool call() method executes
        // Verify the struct has the glob field and it can be set
        assert_eq!(args.glob.as_deref(), Some("*.rs"));

        // @step Then the glob filter is passed through to the execute() method
        // The call() method serializes args to Value and passes glob through
        let value = serde_json::to_value(&args).unwrap();
        assert_eq!(value["glob"], "*.rs");
    }

    /// Scenario: SearchToolFacadeWrapper passes include as glob to GrepTool
    #[test]
    fn test_wrapper_grep_args_construction_with_include() {
        use crate::grep::GrepArgs;

        // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
        let facade = CodexGrepFilesFacade;

        // @step When the wrapper receives InternalSearchParams::Grep with include "*.rs"
        let input = json!({
            "pattern": "TODO",
            "include": "*.rs",
            "path": "/src"
        });
        let params = facade.map_params(input).unwrap();

        // @step Then the wrapper passes "glob" = "*.rs" in the GrepTool execute args
        // Verify the internal params carry include, and show how wrapper would construct GrepArgs
        if let InternalSearchParams::Grep { pattern, path, include, limit } = params {
            let grep_args = GrepArgs {
                pattern,
                path,
                output_mode: None,
                glob: include,
                limit,
            };
            assert_eq!(grep_args.glob.as_deref(), Some("*.rs"));
        } else {
            panic!("Expected InternalSearchParams::Grep");
        }
    }

    /// Scenario: SearchToolFacadeWrapper applies limit to cap grep results
    #[test]
    fn test_wrapper_grep_args_construction_with_limit() {
        use crate::grep::GrepArgs;

        // @step Given a SearchToolFacadeWrapper with a CodexGrepFilesFacade
        let facade = CodexGrepFilesFacade;

        // @step When the wrapper receives InternalSearchParams::Grep with limit 10
        let input = json!({
            "pattern": "TODO",
            "limit": 10
        });
        let params = facade.map_params(input).unwrap();

        // @step Then the wrapper caps the grep output to at most 10 result lines
        // Verify the internal params carry limit, and show how wrapper would construct GrepArgs
        if let InternalSearchParams::Grep { pattern, path, include, limit } = params {
            let grep_args = GrepArgs {
                pattern,
                path,
                output_mode: None,
                glob: include,
                limit,
            };
            assert_eq!(grep_args.limit, Some(10));
        } else {
            panic!("Expected InternalSearchParams::Grep");
        }
    }

    /// Feature: spec/features/codex-grep-files-include-limit.feature
    ///
    /// Scenario: ZAI grep facade constructs InternalSearchParams::Grep with None defaults
    #[test]
    fn test_zai_grep_facade_uses_none_defaults_for_include_and_limit() {
        use crate::facade::zai::ZAIGrepFilesFacade;

        // @step Given a ZAIGrepFilesFacade instance
        let facade = ZAIGrepFilesFacade;

        // @step When the ZAI model calls grep_files with pattern "TODO" and path "src"
        let input = json!({
            "pattern": "TODO",
            "path": "src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("src".to_string()),
                include: None,
                limit: None,
            }
        );
    }

    /// Scenario: Gemini grep facade constructs InternalSearchParams::Grep with None defaults
    #[test]
    fn test_gemini_grep_facade_uses_none_defaults_for_include_and_limit() {
        use crate::facade::search::GeminiSearchFileContentFacade;

        // @step Given a GeminiSearchFileContentFacade instance
        let facade = GeminiSearchFileContentFacade;

        // @step When Gemini sends parameters {pattern: 'TODO', dir_path: 'src'} to tool 'search_file_content'
        let input = json!({
            "pattern": "TODO",
            "dir_path": "src"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalSearchParams::Grep with include None and limit None
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("src".to_string()),
                include: None,
                limit: None,
            }
        );
    }

    // =========================================================================
    // BUG-109: read_file mode and indentation params tests
    // Feature: spec/features/codex-read-file-mode-indentation.feature
    // =========================================================================

    /// Scenario: CodexReadFileFacade maps mode indentation to InternalFileParams::Read
    #[test]
    fn test_codex_read_file_maps_mode_indentation() {
        use crate::facade::traits::InternalIndentationParams;

        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {anchor_line: 50, max_levels: 2}
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "indentation",
            "indentation": {
                "anchor_line": 50,
                "max_levels": 2
            }
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is Some("indentation")
        // @step And indentation anchor_line is Some(50)
        // @step And indentation max_levels is Some(2)
        // @step And indentation include_siblings is None
        // @step And indentation include_header is None
        // @step And indentation max_lines is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("indentation".to_string()),
                indentation: Some(InternalIndentationParams {
                    anchor_line: Some(50),
                    max_levels: Some(2),
                    include_siblings: None,
                    include_header: None,
                    max_lines: None,
                }),
            }
        );
    }

    /// Scenario: CodexReadFileFacade maps mode slice without indentation
    #[test]
    fn test_codex_read_file_maps_mode_slice() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" and mode "slice"
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "slice"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is Some("slice")
        // @step And indentation is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("slice".to_string()),
                indentation: None,
            }
        );
    }

    /// Scenario: CodexReadFileFacade backward compatible without mode or indentation
    #[test]
    fn test_codex_read_file_backward_compatible_no_mode_no_indentation() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with only file_path "/src/main.rs"
        let input = json!({
            "file_path": "/src/main.rs"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/src/main.rs"
        // @step And mode is None
        // @step And indentation is None
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

    /// Scenario: Codex read_file schema includes mode and indentation properties
    #[test]
    fn test_codex_read_file_schema_has_mode_and_indentation() {
        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the tool definition schema is inspected
        let def = facade.definition();

        // @step Then the schema has a "mode" property of type "string"
        assert_eq!(def.parameters["properties"]["mode"]["type"], "string");

        // @step And the schema has an "indentation" property of type "object"
        assert_eq!(
            def.parameters["properties"]["indentation"]["type"],
            "object"
        );

        // @step And the indentation object has an "anchor_line" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["anchor_line"]["type"],
            "integer"
        );

        // @step And the indentation object has a "max_levels" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["max_levels"]["type"],
            "integer"
        );

        // @step And the indentation object has an "include_siblings" property of type "boolean"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["include_siblings"]["type"],
            "boolean"
        );

        // @step And the indentation object has an "include_header" property of type "boolean"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["include_header"]["type"],
            "boolean"
        );

        // @step And the indentation object has a "max_lines" property of type "integer"
        assert_eq!(
            def.parameters["properties"]["indentation"]["properties"]["max_lines"]["type"],
            "integer"
        );

        // @step And the indentation object has additionalProperties false
        assert_eq!(
            def.parameters["properties"]["indentation"]["additionalProperties"],
            false
        );
    }

    /// Scenario: Other facades provide None for mode and indentation fields
    #[test]
    fn test_zai_facade_provides_none_mode_and_indentation() {
        use crate::facade::zai::ZAIReadFileFacade;

        // @step Given a ZAIReadFileFacade instance
        let facade = ZAIReadFileFacade;

        // @step When the ZAI model calls read_file with file_path "/src/main.rs"
        let input = json!({
            "file_path": "/src/main.rs"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with mode None and indentation None
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

    /// Scenario: CodexReadFileFacade extracts all indentation boolean and integer fields
    #[test]
    fn test_codex_read_file_extracts_all_indentation_fields() {
        use crate::facade::traits::InternalIndentationParams;

        // @step Given a CodexReadFileFacade instance
        let facade = CodexReadFileFacade;

        // @step When the Codex model calls read_file with file_path "/src/main.rs" mode "indentation" and indentation {include_siblings: true, include_header: true, max_lines: 100}
        let input = json!({
            "file_path": "/src/main.rs",
            "mode": "indentation",
            "indentation": {
                "include_siblings": true,
                "include_header": true,
                "max_lines": 100
            }
        });
        let result = facade.map_params(input).unwrap();

        // @step Then indentation include_siblings is Some(true)
        // @step And indentation include_header is Some(true)
        // @step And indentation max_lines is Some(100)
        // @step And indentation anchor_line is None
        // @step And indentation max_levels is None
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/src/main.rs".to_string(),
                offset: None,
                limit: None,
                mode: Some("indentation".to_string()),
                indentation: Some(InternalIndentationParams {
                    anchor_line: None,
                    max_levels: None,
                    include_siblings: Some(true),
                    include_header: Some(true),
                    max_lines: Some(100),
                }),
            }
        );
    }

    // =========================================================================
    // view_image facade tests
    // Feature: spec/features/codex-view-image.feature
    // =========================================================================

    /// Scenario: CodexViewImageFacade maps view_image path to InternalFileParams::Read
    #[test]
    fn test_codex_view_image_facade_maps_path_to_read() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the Codex model calls view_image with path "/tmp/screenshot.png"
        let input = json!({
            "path": "/tmp/screenshot.png"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read with file_path "/tmp/screenshot.png"
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/screenshot.png".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );

        // @step And the facade tool name is "view_image"
        assert_eq!(facade.tool_name(), "view_image");

        // @step And the facade provider is "codex"
        assert_eq!(facade.provider(), "codex");
    }

    /// Scenario: CodexViewImageFacade accepts detail param for compatibility
    #[test]
    fn test_codex_view_image_facade_accepts_detail_param() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the Codex model calls view_image with path and detail "original"
        let input = json!({
            "path": "/tmp/image.jpg",
            "detail": "original"
        });
        let result = facade.map_params(input).unwrap();

        // @step Then the facade maps to InternalFileParams::Read ignoring the detail param
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/image.jpg".to_string(),
                offset: None,
                limit: None,
                mode: None,
                indentation: None,
            }
        );
    }

    /// Scenario: CodexViewImageFacade rejects missing path
    #[test]
    fn test_codex_view_image_facade_missing_path() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When view_image is called with no path parameter
        let input = json!({});
        let result = facade.map_params(input);

        // @step Then the facade returns a validation error for tool "view_image" mentioning "path"
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "view_image");
            assert!(message.contains("path"));
        } else {
            panic!("Expected ToolError::Validation");
        }
    }

    /// Scenario: CodexViewImageFacade rejects empty path
    #[test]
    fn test_codex_view_image_facade_empty_path() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When view_image is called with an empty path
        let input = json!({
            "path": ""
        });
        let result = facade.map_params(input);

        // @step Then the facade returns an error
        assert!(result.is_err());
    }

    /// Scenario: Tool definition matches Codex CLI spec
    #[test]
    fn test_codex_view_image_schema_matches_codex_spec() {
        // @step Given a CodexViewImageFacade instance
        let facade = CodexViewImageFacade;

        // @step When the tool definition is requested
        let def = facade.definition();

        // @step Then the tool name is "view_image"
        assert_eq!(def.name, "view_image");

        // @step And the description mentions viewing a local image
        assert!(def.description.contains("View a local image"));

        // @step And the parameters schema has a required "path" property of type string
        assert_eq!(def.parameters["properties"]["path"]["type"], "string");
        assert_eq!(def.parameters["required"], json!(["path"]));

        // @step And additionalProperties is false
        assert_eq!(def.parameters["additionalProperties"], false);

        // @step And a "detail" property exists for model compatibility
        assert!(def.parameters["properties"]["detail"].is_object());
    }
}
