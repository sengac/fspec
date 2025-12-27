//! Search operation facades for different LLM providers.
//!
//! These facades adapt the GrepTool and GlobTool interfaces for provider-specific
//! tool naming and parameter schemas.

use super::traits::{InternalSearchParams, SearchToolFacade, ToolDefinition};
use crate::ToolError;
use serde_json::{json, Value};

/// Gemini-specific facade for content search (grep).
///
/// Maps Gemini's `search_file_content` tool with flat `{pattern, dir_path}` schema
/// to the internal GrepTool parameters.
pub struct GeminiSearchFileContentFacade;

impl SearchToolFacade for GeminiSearchFileContentFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "search_file_content"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_file_content".to_string(),
            description: "Search for text patterns in file contents using regex".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The regex pattern to search for"
                    },
                    "dir_path": {
                        "type": "string",
                        "description": "Directory or file to search in (optional, defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "search_file_content",
                message: "Missing 'pattern' field".to_string(),
            })?
            .to_string();

        let path = input
            .get("dir_path")
            .and_then(|p| p.as_str())
            .map(String::from);

        Ok(InternalSearchParams::Grep { pattern, path })
    }
}

/// Gemini-specific facade for file pattern matching (glob).
///
/// Maps Gemini's `glob` tool with flat `{pattern, dir_path}` schema
/// to the internal GlobTool parameters.
pub struct GeminiGlobFacade;

impl SearchToolFacade for GeminiGlobFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "glob"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "glob".to_string(),
            description: "Find files matching a glob pattern".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "pattern": {
                        "type": "string",
                        "description": "The glob pattern to match files (e.g., '**/*.rs')"
                    },
                    "dir_path": {
                        "type": "string",
                        "description": "Directory to search in (optional, defaults to current directory)"
                    }
                },
                "required": ["pattern"]
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalSearchParams, ToolError> {
        let pattern = input
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::Validation {
                tool: "glob",
                message: "Missing 'pattern' field".to_string(),
            })?
            .to_string();

        let path = input
            .get("dir_path")
            .and_then(|p| p.as_str())
            .map(String::from);

        Ok(InternalSearchParams::Glob { pattern, path })
    }
}
