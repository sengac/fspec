//! Write tool implementation
//!
//! Writes content to files, creating parent directories as needed.
//! Uses tokio::fs for non-blocking async I/O.

use super::error::ToolError;
use super::validation::{create_parent_dirs, require_absolute_path, write_file_contents};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Write tool for writing file contents
pub struct WriteTool;

impl WriteTool {
    /// Create a new Write tool instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

/// Arguments for Write tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WriteArgs {
    /// Absolute path to the file to write
    pub file_path: String,
    /// Content to write to the file
    pub content: String,
}

impl rig::tool::Tool for WriteTool {
    const NAME: &'static str = "Write";

    type Error = ToolError;
    type Args = WriteArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Write".to_string(),
            description:
                "Write content to a file (creates or overwrites). Requires absolute path. \
                Creates parent directories if they don't exist."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WriteArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&args.file_path).map_err(|e| ToolError::Validation {
            tool: "write",
            message: e.content,
        })?;

        // Create parent directories if needed (async)
        create_parent_dirs(path)
            .await
            .map_err(|e| ToolError::File {
                tool: "write",
                message: e.content,
            })?;

        // Write file (async)
        write_file_contents(path, &args.content)
            .await
            .map_err(|e| ToolError::File {
                tool: "write",
                message: e.content,
            })?;

        Ok(format!("Successfully wrote to {}", args.file_path))
    }
}
