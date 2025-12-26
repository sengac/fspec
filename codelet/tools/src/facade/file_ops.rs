//! File operation tool facades for different providers.
//!
//! Provides Gemini-compatible facades for read, write, and edit operations
//! with flat JSON schemas that Gemini understands.

use super::traits::{FileToolFacade, InternalFileParams, ToolDefinition};
use crate::ToolError;
use serde_json::{json, Value};

/// Gemini-specific read file facade.
///
/// Gemini uses `read_file` with a flat `{path}` schema.
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
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the file to read"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let path = input
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "read_file",
                message: "Missing 'path' field".to_string(),
            })?
            .to_string();

        Ok(InternalFileParams::Read {
            file_path: path,
            offset: None,
            limit: None,
        })
    }
}

/// Gemini-specific write file facade.
///
/// Gemini uses `write_file` with a flat `{path, content}` schema.
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
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the file to write"
                    },
                    "content": {
                        "type": "string",
                        "description": "The content to write to the file"
                    }
                },
                "required": ["path", "content"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let path = input
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "write_file",
                message: "Missing 'path' field".to_string(),
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
            file_path: path,
            content,
        })
    }
}

/// Gemini-specific replace/edit facade.
///
/// Gemini uses `replace` with a flat `{path, old_text, new_text}` schema.
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
                    "path": {
                        "type": "string",
                        "description": "The absolute path to the file to edit"
                    },
                    "old_text": {
                        "type": "string",
                        "description": "The text to find and replace"
                    },
                    "new_text": {
                        "type": "string",
                        "description": "The replacement text"
                    }
                },
                "required": ["path", "old_text", "new_text"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalFileParams, ToolError> {
        let path = input
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'path' field".to_string(),
            })?
            .to_string();

        let old_text = input
            .get("old_text")
            .and_then(|o| o.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'old_text' field".to_string(),
            })?
            .to_string();

        let new_text = input
            .get("new_text")
            .and_then(|n| n.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "replace",
                message: "Missing 'new_text' field".to_string(),
            })?
            .to_string();

        Ok(InternalFileParams::Edit {
            file_path: path,
            old_string: old_text,
            new_string: new_text,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_read_file_facade_maps_path() {
        let facade = GeminiReadFileFacade;
        let input = json!({
            "path": "/tmp/test.txt"
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
    fn test_gemini_write_file_facade_maps_path_and_content() {
        let facade = GeminiWriteFileFacade;
        let input = json!({
            "path": "/tmp/test.txt",
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
            "path": "/tmp/test.txt",
            "old_text": "foo",
            "new_text": "bar"
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
        assert!(def.parameters["properties"].get("path").is_some());
        assert!(def.parameters.get("oneOf").is_none());
        assert!(def.parameters["properties"].get("action").is_none());
    }

    #[test]
    fn test_gemini_write_file_facade_has_flat_schema() {
        let facade = GeminiWriteFileFacade;
        let def = facade.definition();

        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("path").is_some());
        assert!(def.parameters["properties"].get("content").is_some());
        assert!(def.parameters.get("oneOf").is_none());
    }

    #[test]
    fn test_gemini_replace_facade_has_flat_schema() {
        let facade = GeminiReplaceFacade;
        let def = facade.definition();

        assert!(def.parameters.get("properties").is_some());
        assert!(def.parameters["properties"].get("path").is_some());
        assert!(def.parameters["properties"].get("old_text").is_some());
        assert!(def.parameters["properties"].get("new_text").is_some());
        assert!(def.parameters.get("oneOf").is_none());
    }
}
