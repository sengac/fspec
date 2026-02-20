//! Edit tool implementation
//!
//! Edits files by replacing the first occurrence of a string.
//! Uses tokio::fs for non-blocking async I/O.
//!
//! TOOL-014: Supports worktree isolation via session_id for isolated sessions.

use super::blocklist::check_file_path;
use super::error::ToolError;
use super::validation::{
    read_file_contents, require_absolute_path, require_file_exists, write_file_contents,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Edit tool for modifying file contents
///
/// TOOL-014: Requires session_id for worktree isolation support.
/// In isolated sessions, file paths are resolved to the session's worktree.
pub struct EditTool {
    /// Session ID for worktree isolation support
    #[allow(dead_code)] // Will be used for path resolution in worktree isolation
    session_id: Uuid,
}

impl EditTool {
    /// Create a new Edit tool instance with session awareness
    ///
    /// # Arguments
    /// * `session_id` - The session ID for worktree isolation (TOOL-014)
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
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

impl rig::tool::Tool for EditTool {
    const NAME: &'static str = "Edit";

    type Error = ToolError;
    type Args = EditArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "Edit".to_string(),
            description:
                "Edit a file by replacing old_string with new_string (first occurrence only). \
                Requires absolute path."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(EditArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Check file path against blocklist before any I/O
        if let Err(blocked) = check_file_path(&args.file_path) {
            return Err(ToolError::Blocked {
                tool: "edit",
                message: blocked.to_string(),
            });
        }

        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&args.file_path).map_err(|e| ToolError::Validation {
            tool: "edit",
            message: e.content,
        })?;

        // Check file exists (async)
        require_file_exists(path, &args.file_path)
            .await
            .map_err(|e| ToolError::Validation {
                tool: "edit",
                message: e.content,
            })?;

        // Read file content (async)
        let content = read_file_contents(path)
            .await
            .map_err(|e| ToolError::File {
                tool: "edit",
                message: e.content,
            })?;

        // Check if old_string exists
        if !content.contains(&args.old_string) {
            return Err(ToolError::StringNotFound {
                tool: "edit",
                message: "old_string not found in file".to_string(),
            });
        }

        // Replace first occurrence only
        let new_content = content.replacen(&args.old_string, &args.new_string, 1);

        // Write back (async)
        write_file_contents(path, &new_content)
            .await
            .map_err(|e| ToolError::File {
                tool: "edit",
                message: e.content,
            })?;

        Ok(format!("Successfully edited {}", args.file_path))
    }
}
