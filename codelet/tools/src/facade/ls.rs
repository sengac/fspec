//! Directory listing facade for different LLM providers.
//!
//! This facade adapts the LsTool interface for provider-specific
//! tool naming and parameter schemas.

use super::traits::{InternalLsParams, LsToolFacade, ToolDefinition};
use crate::ToolError;
use serde_json::{json, Value};

/// Gemini-specific facade for directory listing.
///
/// Maps Gemini's `list_directory` tool with flat `{path}` schema
/// to the internal LsTool parameters.
pub struct GeminiListDirectoryFacade;

impl LsToolFacade for GeminiListDirectoryFacade {
    fn provider(&self) -> &'static str {
        "gemini"
    }

    fn tool_name(&self) -> &'static str {
        "list_directory"
    }

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_directory".to_string(),
            description: "List directory contents with file metadata (permissions, size, modification time)".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Directory to list (optional, defaults to current directory)"
                    }
                }
            }),
        }
    }

    fn map_params(&self, input: Value) -> Result<InternalLsParams, ToolError> {
        let path = input.get("path").and_then(|p| p.as_str()).map(String::from);

        Ok(InternalLsParams::List { path })
    }
}
