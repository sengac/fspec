//! Edit tool implementation
//!
//! Edits files by replacing the first occurrence of a string.
//! Uses tokio::fs for non-blocking async I/O.
//!
//! For isolated sessions, file paths are validated and resolved to the worktree
//! to ensure the session cannot edit files outside its isolated environment.

use super::bash_binary_guard::{detect_bash_binary_output, format_file_tool_guard_message};
use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::validation::{require_absolute_path, require_file_exists, write_file_contents};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Edit tool for modifying file contents.
///
/// Requires a session ID for path resolution. In isolated sessions, paths are
/// resolved relative to the session's worktree directory.
pub struct EditTool {
    session_id: Uuid,
}

impl EditTool {
    /// Create a new Edit tool instance.
    ///
    /// # Arguments
    /// * `session_id` - The session ID used for path resolution in isolated sessions
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
        // HOOK-017: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            "Edit",
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Edit",
                message: reason,
            });
        }

        // Validate and resolve path (handles worktree isolation for isolated sessions)
        let resolved_path = validate_and_resolve_path(self.session_id, &args.file_path, "edit")?;
        let file_path_str = resolved_path.to_string_lossy().to_string();

        // Check file path against blocklist before any I/O
        if let Err(blocked) = check_file_path(&file_path_str, self.session_id) {
            return Err(ToolError::Blocked {
                tool: "edit",
                message: blocked.to_string(),
            });
        }

        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&file_path_str).map_err(|e| ToolError::Validation {
            tool: "edit",
            message: e.content,
        })?;

        // Check file exists (async) — BUG-130: may resolve via Unicode fallback
        let resolved_file = require_file_exists(path, &file_path_str)
            .await
            .map_err(|e| ToolError::Validation {
                tool: "edit",
                message: e.content,
            })?;
        let path = resolved_file.as_path();
        let file_path_str = path.to_string_lossy().to_string();

        // Read file as bytes first (BUG-143). We run the binary-guard on the raw
        // bytes BEFORE attempting UTF-8 decode so that the agent gets a
        // structured "this is a binary file" error naming the format (PNG/PDF/…)
        // rather than a confusing generic UTF-8 decode failure.
        let bytes = tokio::fs::read(path).await.map_err(|e| ToolError::File {
            tool: "edit",
            message: format!("Error reading file: {e}"),
        })?;

        if let Some(kind) = detect_bash_binary_output(&bytes) {
            return Err(ToolError::Validation {
                tool: "edit",
                message: format_file_tool_guard_message("Edit", kind),
            });
        }

        let content = String::from_utf8(bytes).map_err(|e| ToolError::File {
            tool: "edit",
            message: format!("Error reading file: {e}"),
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

        Ok(format!("Successfully edited {file_path_str}"))
    }
}
