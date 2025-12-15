//! Write tool implementation
//!
//! Writes content to files, creating parent directories as needed.

use super::validation::require_absolute_path;
use super::{Tool, ToolOutput, ToolParameters};
use anyhow::Result;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use tracing::debug;

/// Write tool for writing file contents
pub struct WriteTool {
    parameters: ToolParameters,
}

impl WriteTool {
    /// Create a new Write tool instance
    pub fn new() -> Self {
        let mut properties = serde_json::Map::new();

        properties.insert(
            "file_path".to_string(),
            json!({
                "type": "string",
                "description": "Absolute path to the file to write"
            }),
        );

        properties.insert(
            "content".to_string(),
            json!({
                "type": "string",
                "description": "Content to write to the file"
            }),
        );

        Self {
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties,
                required: vec!["file_path".to_string(), "content".to_string()],
            },
        }
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates parent directories if they don't exist."
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        debug!(tool = "Write", input = ?args, "Executing tool");

        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");

        // Validate absolute path
        let path = match require_absolute_path(file_path) {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Ok(ToolOutput::error(format!(
                        "Error creating directories: {e}"
                    )));
                }
            }
        }

        // Write file
        match fs::write(path, content) {
            Ok(()) => {
                // Format diff output for CLI-007 diff rendering
                // Show each line as an addition
                let diff_lines: Vec<String> =
                    content.lines().map(|line| format!("+ {line}")).collect();
                let diff_output = format!("File: {}\n{}", file_path, diff_lines.join("\n"));
                Ok(ToolOutput::success(diff_output))
            }
            Err(e) => Ok(ToolOutput::error(format!("Error writing file: {e}"))),
        }
    }
}

// ========================================
// Rig Tool Implementation (REFAC-004)
// ========================================

/// Arguments for Write tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct WriteArgs {
    /// Absolute path to the file to write
    pub file_path: String,
    /// Content to write to the file
    pub content: String,
}

/// Error type for Write tool
#[derive(Debug, thiserror::Error)]
pub enum WriteError {
    #[error("File error: {0}")]
    FileError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

impl rig::tool::Tool for WriteTool {
    const NAME: &'static str = "write";

    type Error = WriteError;
    type Args = WriteArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "write".to_string(),
            description: "Write content to a file. Creates parent directories if they don't exist."
                .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(WriteArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate absolute path
        let path = require_absolute_path(&args.file_path)
            .map_err(|e| WriteError::ValidationError(e.content))?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).map_err(|e| {
                    WriteError::FileError(format!("Error creating directories: {e}"))
                })?;
            }
        }

        // Write file
        fs::write(path, &args.content)
            .map_err(|e| WriteError::FileError(format!("Error writing file: {e}")))?;

        Ok(format!("Successfully wrote to {}", args.file_path))
    }
}
