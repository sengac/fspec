//! Write tool implementation
//!
//! Writes content to files, creating parent directories as needed.
//! Uses tokio::fs for non-blocking async I/O.
//!
//! For isolated sessions, file paths are validated and resolved to the worktree
//! to ensure the session cannot write files outside its isolated environment.

use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::validation::{create_parent_dirs, require_absolute_path, write_file_contents};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

/// Write tool for writing file contents.
///
/// Requires a session ID for path resolution. In isolated sessions, paths are
/// resolved relative to the session's worktree directory.
pub struct WriteTool {
    session_id: Uuid,
}

impl WriteTool {
    /// Create a new Write tool instance.
    ///
    /// # Arguments
    /// * `session_id` - The session ID used for path resolution in isolated sessions
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
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
        // HOOK-017: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            "Write",
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "Write",
                message: reason,
            });
        }

        // Validate and resolve path (handles worktree isolation for isolated sessions)
        let resolved_path = validate_and_resolve_path(self.session_id, &args.file_path, "write")?;
        let file_path_str = resolved_path.to_string_lossy().to_string();

        // Check file path against blocklist before any I/O
        if let Err(blocked) = check_file_path(&file_path_str, self.session_id) {
            return Err(ToolError::Blocked {
                tool: "write",
                message: blocked.to_string(),
            });
        }

        // Validate absolute path (sync - no I/O)
        let path = require_absolute_path(&file_path_str).map_err(|e| ToolError::Validation {
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

        Ok(format!("Successfully wrote to {file_path_str}"))
    }
}
