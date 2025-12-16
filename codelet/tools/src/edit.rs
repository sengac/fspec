//! Edit tool implementation
//!
//! Edits files by replacing the first occurrence of a string.

use super::validation::{read_file_contents, require_absolute_path, require_file_exists};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;

/// Edit tool for modifying file contents
pub struct EditTool;

impl EditTool {
    /// Create a new Edit tool instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for EditTool {
    fn default() -> Self {
        Self::new()
    }
}

// rig::tool::Tool implementation

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
                "Edit a file by replacing old_string with new_string (first occurrence only). \
                Requires absolute path."
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
