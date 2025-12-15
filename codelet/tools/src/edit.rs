//! Edit tool implementation
//!
//! Edits files by replacing the first occurrence of a string.

use super::validation::{read_file_contents, require_absolute_path, require_file_exists};
use super::{Tool, ToolOutput, ToolParameters};
use anyhow::Result;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use tracing::debug;

/// Edit tool for modifying file contents
pub struct EditTool {
    parameters: ToolParameters,
}

impl EditTool {
    /// Create a new Edit tool instance
    pub fn new() -> Self {
        let mut properties = serde_json::Map::new();

        properties.insert(
            "file_path".to_string(),
            json!({
                "type": "string",
                "description": "Absolute path to the file to edit"
            }),
        );

        properties.insert(
            "old_string".to_string(),
            json!({
                "type": "string",
                "description": "String to find and replace (first occurrence only)"
            }),
        );

        properties.insert(
            "new_string".to_string(),
            json!({
                "type": "string",
                "description": "String to replace with"
            }),
        );

        Self {
            parameters: ToolParameters {
                schema_type: "object".to_string(),
                properties,
                required: vec![
                    "file_path".to_string(),
                    "old_string".to_string(),
                    "new_string".to_string(),
                ],
            },
        }
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Edit a file by replacing the first occurrence of old_string with new_string."
    }

    fn parameters(&self) -> &ToolParameters {
        &self.parameters
    }

    async fn execute(&self, args: Value) -> Result<ToolOutput> {
        debug!(tool = "Edit", input = ?args, "Executing tool");
        let file_path = args.get("file_path").and_then(|v| v.as_str()).unwrap_or("");

        let old_string = args
            .get("old_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let new_string = args
            .get("new_string")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Validate absolute path
        let path = match require_absolute_path(file_path) {
            Ok(p) => p,
            Err(e) => return Ok(e),
        };

        // Check file exists
        if let Err(e) = require_file_exists(path, file_path) {
            return Ok(e);
        }

        // Read file content
        let content = match read_file_contents(path) {
            Ok(c) => c,
            Err(e) => return Ok(e),
        };

        // Check if old_string exists
        if !content.contains(old_string) {
            return Ok(ToolOutput::error(
                "Error: old_string not found in file".to_string(),
            ));
        }

        // Replace first occurrence only
        let new_content = content.replacen(old_string, new_string, 1);

        // Write back
        match fs::write(path, &new_content) {
            Ok(()) => {
                // Format diff output for CLI-007 diff rendering
                let diff_output =
                    format!("File: {file_path}\n- {old_string}\n+ {new_string}");
                Ok(ToolOutput::success(diff_output))
            }
            Err(e) => Ok(ToolOutput::error(format!("Error writing file: {e}"))),
        }
    }
}

// ========================================
// Rig Tool Implementation (REFAC-004)
// ========================================

/// Arguments for Edit tool (rig::tool::Tool)
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct EditArgs {
    /// Absolute path to the file to edit
    pub file_path: String,
    /// String to find and replace (first occurrence only)
    pub old_string: String,
    /// String to replace with
    pub new_string: String,
}

/// Error type for Edit tool
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error("File error: {0}")]
    FileError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("String not found: {0}")]
    StringNotFound(String),
}

impl rig::tool::Tool for EditTool {
    const NAME: &'static str = "edit";

    type Error = EditError;
    type Args = EditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "edit".to_string(),
            description:
                "Edit a file by replacing the first occurrence of old_string with new_string."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(EditArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Validate absolute path
        let path = require_absolute_path(&args.file_path)
            .map_err(|e| EditError::ValidationError(e.content))?;

        // Check file exists
        require_file_exists(path, &args.file_path)
            .map_err(|e| EditError::ValidationError(e.content))?;

        // Read file content
        let content = read_file_contents(path).map_err(|e| EditError::FileError(e.content))?;

        // Check if old_string exists
        if !content.contains(&args.old_string) {
            return Err(EditError::StringNotFound(
                "old_string not found in file".to_string(),
            ));
        }

        // Replace first occurrence only
        let new_content = content.replacen(&args.old_string, &args.new_string, 1);

        // Write back
        fs::write(path, &new_content)
            .map_err(|e| EditError::FileError(format!("Error writing file: {e}")))?;

        Ok(format!("Successfully edited {}", args.file_path))
    }
}
