//! Z.AI/GLM-specific tool facades.
//!
//! These facades adapt the internal tool interfaces for Z.AI GLM models,
//! using OpenAI-compatible but GLM-optimized tool naming and parameter schemas.
//!
//! GLM models work best with:
//! - snake_case tool names (e.g., `list_dir`, `read_file`)
//! - Flat JSON schemas with explicit `required` and `default` values
//! - `additionalProperties: false` to prevent extra fields
//! - Clear, concise descriptions

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
            message: format!("Missing or empty required '{}' field", field),
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
    input.get(field).and_then(|v| v.as_u64()).map(|n| n as usize)
}

// ============================================================================
// Directory Listing Facade
// ============================================================================

/// Z.AI/GLM-specific facade for directory listing.
///
/// Maps GLM's `list_dir` tool to the internal LsTool parameters.
/// Uses snake_case naming and flat schema that GLM understands.
pub struct ZAIListDirFacade;

impl LsToolFacade for ZAIListDirFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "list_dir"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_dir".to_string(),
            description: "List directory contents. Returns file/directory names with metadata (size, modification time). Defaults to current directory if path not specified.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory path to list (relative or absolute). Defaults to current directory.",
                        "default": "."
                    }
                },
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError> {
        let path = extract_optional_string(&input, "path");
        Ok(InternalLsParams::List { path })
    }
}

// ============================================================================
// File Operation Facades
// ============================================================================

/// Z.AI/GLM-specific read file facade.
///
/// Uses `read_file` with flat schema that GLM understands.
pub struct ZAIReadFileFacade;

impl FileToolFacade for ZAIReadFileFacade {
    fn provider(&self) -> &'static str {
        "zai"
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
                        "description": "Line number to start reading from (1-based, optional)"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to read (optional)"
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

/// Z.AI/GLM-specific write file facade.
///
/// Uses `write_file` with flat schema that GLM understands.
pub struct ZAIWriteFileFacade;

impl FileToolFacade for ZAIWriteFileFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "write_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file. Creates parent directories if needed. Overwrites existing files.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write to the file"
                    }
                },
                "required": ["file_path", "content"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = extract_required_string(&input, "file_path", "write_file")?;
        let content = extract_required_string(&input, "content", "write_file")?;

        Ok(InternalFileParams::Write { file_path, content })
    }
}

/// Z.AI/GLM-specific edit file facade.
///
/// Uses `edit_file` with flat schema that GLM understands.
pub struct ZAIEditFileFacade;

impl FileToolFacade for ZAIEditFileFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "edit_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "edit_file".to_string(),
            description: "Edit a file by replacing exact text. Finds first occurrence of old_string and replaces with new_string.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "Absolute path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "Exact text to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "Replacement text"
                    }
                },
                "required": ["file_path", "old_string", "new_string"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = extract_required_string(&input, "file_path", "edit_file")?;
        let old_string = extract_required_string(&input, "old_string", "edit_file")?;
        let new_string = extract_required_string(&input, "new_string", "edit_file")?;

        Ok(InternalFileParams::Edit {
            file_path,
            old_string,
            new_string,
        })
    }
}

// ============================================================================
// Bash/Shell Facade
// ============================================================================

/// Z.AI/GLM-specific bash facade.
///
/// Uses `run_command` with flat schema that GLM understands.
pub struct ZAIRunCommandFacade;

impl BashToolFacade for ZAIRunCommandFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "run_command"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "run_command".to_string(),
            description: "Execute a shell command. Returns stdout on success, stderr on failure. Commands run in the current working directory.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute"
                    }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalBashParams, ToolError> {
        let command = extract_required_string(&input, "command", "run_command")?;
        Ok(InternalBashParams::Execute { command })
    }
}

// ============================================================================
// Search Facades
// ============================================================================

/// Z.AI/GLM-specific grep facade.
///
/// Uses `grep_files` with flat schema that GLM understands.
pub struct ZAIGrepFilesFacade;

impl SearchToolFacade for ZAIGrepFilesFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "grep_files"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "grep_files".to_string(),
            description: "Search file contents using regex pattern. Returns matching lines with file paths and line numbers.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Regex pattern to search for"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (defaults to current directory)",
                        "default": "."
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

/// Z.AI/GLM-specific glob facade.
///
/// Uses `find_files` with flat schema that GLM understands.
pub struct ZAIFindFilesFacade;

impl SearchToolFacade for ZAIFindFilesFacade {
    fn provider(&self) -> &'static str {
        "zai"
    }

    fn tool_name(&self) -> &'static str {
        "find_files"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "find_files".to_string(),
            description: "Find files matching a glob pattern. Returns list of matching file paths.".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "Glob pattern to match (e.g., '**/*.rs', 'src/**/*.ts')"
                    },
                    "path": {
                        "type": "string",
                        "description": "Directory to search in (defaults to current directory)",
                        "default": "."
                    }
                },
                "required": ["pattern"],
                "additionalProperties": false
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = extract_required_string(&input, "pattern", "find_files")?;
        let path = extract_optional_string(&input, "path");

        Ok(InternalSearchParams::Glob { pattern, path })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // list_dir tests
    // =========================================================================

    #[test]
    fn test_zai_list_dir_facade_with_path() {
        let facade = ZAIListDirFacade;
        let input = json!({
            "path": "/tmp/test"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalLsParams::List {
                path: Some("/tmp/test".to_string())
            }
        );
    }

    #[test]
    fn test_zai_list_dir_facade_without_path() {
        let facade = ZAIListDirFacade;
        let input = json!({});

        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None });
    }

    #[test]
    fn test_zai_list_dir_facade_with_empty_path() {
        let facade = ZAIListDirFacade;
        let input = json!({
            "path": ""
        });

        let result = facade.map_params(input).unwrap();
        // Empty path should be treated as None (use default)
        assert_eq!(result, InternalLsParams::List { path: None });
    }

    #[test]
    fn test_zai_list_dir_facade_with_null_path() {
        let facade = ZAIListDirFacade;
        let input = json!({
            "path": null
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(result, InternalLsParams::List { path: None });
    }

    #[test]
    fn test_zai_list_dir_has_default_in_schema() {
        let facade = ZAIListDirFacade;
        let def = facade.definition();

        // Verify the schema has a default value for path
        let path_prop = &def.parameters["properties"]["path"];
        assert_eq!(path_prop["default"], ".");
        // Verify additionalProperties is false
        assert_eq!(def.parameters["additionalProperties"], false);
    }

    // =========================================================================
    // read_file tests
    // =========================================================================

    #[test]
    fn test_zai_read_file_facade() {
        let facade = ZAIReadFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/test.txt".to_string(),
                offset: None,
                limit: None,
            }
        );
    }

    #[test]
    fn test_zai_read_file_facade_with_offset_limit() {
        let facade = ZAIReadFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt",
            "offset": 10,
            "limit": 100
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/test.txt".to_string(),
                offset: Some(10),
                limit: Some(100),
            }
        );
    }

    #[test]
    fn test_zai_read_file_facade_missing_path() {
        let facade = ZAIReadFileFacade;
        let input = json!({});

        let result = facade.map_params(input);
        assert!(result.is_err());
        if let Err(ToolError::Validation { tool, message }) = result {
            assert_eq!(tool, "read_file");
            assert!(message.contains("file_path"));
        }
    }

    #[test]
    fn test_zai_read_file_facade_empty_path() {
        let facade = ZAIReadFileFacade;
        let input = json!({
            "file_path": ""
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // write_file tests
    // =========================================================================

    #[test]
    fn test_zai_write_file_facade() {
        let facade = ZAIWriteFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt",
            "content": "hello world"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalFileParams::Write {
                file_path: "/tmp/test.txt".to_string(),
                content: "hello world".to_string(),
            }
        );
    }

    #[test]
    fn test_zai_write_file_facade_missing_content() {
        let facade = ZAIWriteFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // edit_file tests
    // =========================================================================

    #[test]
    fn test_zai_edit_file_facade() {
        let facade = ZAIEditFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt",
            "old_string": "foo",
            "new_string": "bar"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalFileParams::Edit {
                file_path: "/tmp/test.txt".to_string(),
                old_string: "foo".to_string(),
                new_string: "bar".to_string(),
            }
        );
    }

    #[test]
    fn test_zai_edit_file_facade_missing_old_string() {
        let facade = ZAIEditFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt",
            "new_string": "bar"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // run_command tests
    // =========================================================================

    #[test]
    fn test_zai_run_command_facade() {
        let facade = ZAIRunCommandFacade;
        let input = json!({
            "command": "ls -la"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalBashParams::Execute {
                command: "ls -la".to_string()
            }
        );
    }

    #[test]
    fn test_zai_run_command_facade_missing_command() {
        let facade = ZAIRunCommandFacade;
        let input = json!({});

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_zai_run_command_facade_empty_command() {
        let facade = ZAIRunCommandFacade;
        let input = json!({
            "command": ""
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // grep_files tests
    // =========================================================================

    #[test]
    fn test_zai_grep_files_facade() {
        let facade = ZAIGrepFilesFacade;
        let input = json!({
            "pattern": "TODO",
            "path": "src"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalSearchParams::Grep {
                pattern: "TODO".to_string(),
                path: Some("src".to_string())
            }
        );
    }

    #[test]
    fn test_zai_grep_files_facade_no_path() {
        let facade = ZAIGrepFilesFacade;
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
    fn test_zai_grep_files_facade_missing_pattern() {
        let facade = ZAIGrepFilesFacade;
        let input = json!({
            "path": "src"
        });

        let result = facade.map_params(input);
        assert!(result.is_err());
    }

    // =========================================================================
    // find_files tests
    // =========================================================================

    #[test]
    fn test_zai_find_files_facade() {
        let facade = ZAIFindFilesFacade;
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
    fn test_zai_find_files_facade_with_path() {
        let facade = ZAIFindFilesFacade;
        let input = json!({
            "pattern": "*.ts",
            "path": "src/components"
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalSearchParams::Glob {
                pattern: "*.ts".to_string(),
                path: Some("src/components".to_string())
            }
        );
    }

    // =========================================================================
    // Tool naming tests
    // =========================================================================

    #[test]
    fn test_zai_tools_use_snake_case_names() {
        assert_eq!(ZAIListDirFacade.tool_name(), "list_dir");
        assert_eq!(ZAIReadFileFacade.tool_name(), "read_file");
        assert_eq!(ZAIWriteFileFacade.tool_name(), "write_file");
        assert_eq!(ZAIEditFileFacade.tool_name(), "edit_file");
        assert_eq!(ZAIRunCommandFacade.tool_name(), "run_command");
        assert_eq!(ZAIGrepFilesFacade.tool_name(), "grep_files");
        assert_eq!(ZAIFindFilesFacade.tool_name(), "find_files");
    }

    #[test]
    fn test_zai_tools_provider_name() {
        assert_eq!(ZAIListDirFacade.provider(), "zai");
        assert_eq!(ZAIReadFileFacade.provider(), "zai");
        assert_eq!(ZAIWriteFileFacade.provider(), "zai");
        assert_eq!(ZAIEditFileFacade.provider(), "zai");
        assert_eq!(ZAIRunCommandFacade.provider(), "zai");
        assert_eq!(ZAIGrepFilesFacade.provider(), "zai");
        assert_eq!(ZAIFindFilesFacade.provider(), "zai");
    }

    // =========================================================================
    // Schema validation tests
    // =========================================================================

    #[test]
    fn test_all_zai_schemas_have_additional_properties_false() {
        // All Z.AI tool schemas should have additionalProperties: false
        let facades: Vec<(&str, serde_json::Value)> = vec![
            ("list_dir", ZAIListDirFacade.definition().parameters),
            ("read_file", ZAIReadFileFacade.definition().parameters),
            ("write_file", ZAIWriteFileFacade.definition().parameters),
            ("edit_file", ZAIEditFileFacade.definition().parameters),
            ("run_command", ZAIRunCommandFacade.definition().parameters),
            ("grep_files", ZAIGrepFilesFacade.definition().parameters),
            ("find_files", ZAIFindFilesFacade.definition().parameters),
        ];

        for (name, params) in facades {
            assert_eq!(
                params["additionalProperties"], false,
                "{} should have additionalProperties: false",
                name
            );
        }
    }

    #[test]
    fn test_tools_with_defaults_have_default_values() {
        // Tools with optional path parameters should have default: "."
        let list_dir_params = ZAIListDirFacade.definition().parameters;
        assert_eq!(list_dir_params["properties"]["path"]["default"], ".");

        let grep_params = ZAIGrepFilesFacade.definition().parameters;
        assert_eq!(grep_params["properties"]["path"]["default"], ".");

        let find_params = ZAIFindFilesFacade.definition().parameters;
        assert_eq!(find_params["properties"]["path"]["default"], ".");
    }
}
