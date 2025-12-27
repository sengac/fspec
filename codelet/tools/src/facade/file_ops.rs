//! File operation tool facades for different providers.
//!
//! Provides Gemini-compatible facades for read, write, and edit operations
//! with flat JSON schemas that Gemini understands.

use super::traits::{FileToolFacade, InternalFileParams, ToolDefinition};
use crate::ToolError;
use serde_json::{json, Value};

/// Gemini-specific read file facade.
///
/// Gemini uses `read_file` with a flat `{file_path}` schema.
pub struct GeminiReadFileFacade;

impl FileToolFacade for GeminiReadFileFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "read_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "read_file".to_string(),
            description: "Read the contents of a file from the filesystem".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to read"
                    },
                    "offset": {
                        "type": "number",
                        "description": "Line offset to start reading from (optional)"
                    },
                    "limit": {
                        "type": "number",
                        "description": "Maximum number of lines to read (optional)"
                    }
                },
                "required": ["file_path"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "read_file",
                message: "Missing 'file_path' field".to_string(),
            })?
            .to_string();

        let offset = input
            .get("offset")
            .and_then(|o| o.as_u64())
            .map(|o| o as usize);
        let limit = input
            .get("limit")
            .and_then(|l| l.as_u64())
            .map(|l| l as usize);

        Ok(InternalFileParams::Read {
            file_path,
            offset,
            limit,
        })
    }
}

/// Gemini-specific write file facade.
///
/// Gemini uses `write_file` with a flat `{file_path, content}` schema.
pub struct GeminiWriteFileFacade;

impl FileToolFacade for GeminiWriteFileFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "write_file"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "write_file".to_string(),
            description: "Write content to a file on the filesystem".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["file_path", "content"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "write_file",
                message: "Missing 'file_path' field".to_string(),
            })?
            .to_string();

        let content = input
            .get("content")
            .and_then(|c| c.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "write_file",
                message: "Missing 'content' field".to_string(),
            })?
            .to_string();

        Ok(InternalFileParams::Write {
            file_path,
            content,
        })
    }
}

/// Gemini-specific replace/edit facade.
///
/// Gemini uses `replace` with a flat `{file_path, old_string, new_string}` schema.
pub struct GeminiReplaceFacade;

impl FileToolFacade for GeminiReplaceFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "replace"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "replace".to_string(),
            description: "Replace text in a file".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "file_path": {
                        "type": "string",
                        "description": "The absolute path to the file to edit"
                    },
                    "old_string": {
                        "type": "string",
                        "description": "The exact text to find and replace"
                    },
                    "new_string": {
                        "type": "string",
                        "description": "The replacement text"
                    },
                    "expected_replacements": {
                        "type": "number",
                        "description": "Number of replacements expected (defaults to 1)"
                    }
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let file_path = input
            .get("file_path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'file_path' field".to_string(),
            })?
            .to_string();

        let old_string = input
            .get("old_string")
            .and_then(|o| o.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'old_string' field".to_string(),
            })?
            .to_string();

        let new_string = input
            .get("new_string")
            .and_then(|n| n.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'new_string' field".to_string(),
            })?
            .to_string();

        Ok(InternalFileParams::Edit {
            file_path,
            old_string,
            new_string,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_read_file_facade_maps_file_path() {
        let facade = GeminiReadFileFacade;
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
    fn test_gemini_read_file_facade_maps_offset_and_limit() {
        let facade = GeminiReadFileFacade;
        let input = json!({
            "file_path": "/tmp/test.txt",
            "offset": 10,
            "limit": 50
        });

        let result = facade.map_params(input).unwrap();
        assert_eq!(
            result,
            InternalFileParams::Read {
                file_path: "/tmp/test.txt".to_string(),
                offset: Some(10),
                limit: Some(50),
            }
        );
    }

    #[test]
    fn test_gemini_write_file_facade_maps_file_path_and_content() {
        let facade = GeminiWriteFileFacade;
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
    fn test_gemini_replace_facade_maps_all_fields() {
        let facade = GeminiReplaceFacade;
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
    fn test_gemini_read_file_facade_has_flat_schema() {
        let facade = GeminiReadFileFacade;
        let def = facade.definition();

        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("file_path").is_some());
        assert!(def.parameters["properties"].get("offset").is_some());
        assert!(def.parameters["properties"].get("limit").is_some());
        assert!(def.parameters.get("oneOf").is_none());
        assert!(def.parameters["properties"].get("action").is_none());
    }

    #[test]
    fn test_gemini_write_file_facade_has_flat_schema() {
        let facade = GeminiWriteFileFacade;
        let def = facade.definition();

        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("file_path").is_some());
        assert!(def.parameters["properties"].get("content").is_some());
        assert!(def.parameters.get("oneOf").is_none());
    }

    #[test]
    fn test_gemini_replace_facade_has_flat_schema() {
        let facade = GeminiReplaceFacade;
        let def = facade.definition();

        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("file_path").is_some());
        assert!(def.parameters["properties"].get("old_string").is_some());
        assert!(def.parameters["properties"].get("new_string").is_some());
        assert!(def.parameters["properties"].get("expected_replacements").is_some());
        assert!(def.parameters.get("oneOf").is_none());
    }
}
