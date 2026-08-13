//! Codex `apply_patch` tool implementation.
//!
//! Parses the Codex freeform patch format and delegates to internal file
//! operations (create, edit, delete). This is a standalone `rig::tool::Tool`
//! because `apply_patch` has no equivalent in other providers.
//!
//! Feature: spec/features/codex-apply-patch.feature

mod hunk;
mod parser;

use super::bash_binary_guard::{detect_bash_binary_output, format_file_tool_guard_message};
use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::validation::{
    create_parent_dirs, require_absolute_path, require_file_exists, write_file_contents,
};
use hunk::apply_hunks;
use parser::{parse_patch, PatchOp};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// Path validation helper
// ============================================================================

/// Validate, resolve, blocklist-check, and require absolute for a path.
///
/// Extracts the common 3-step pattern shared by all PatchOp arms:
/// 1. `validate_and_resolve_path` (session isolation)
/// 2. `check_file_path` (blocklist)
/// 3. `require_absolute_path`
fn validate_patch_path(session_id: Uuid, path: &str) -> Result<std::path::PathBuf, ToolError> {
    let resolved = validate_and_resolve_path(session_id, path, "apply_patch")?;
    let p = resolved.to_string_lossy().to_string();
    if let Err(blocked) = check_file_path(&p, session_id) {
        return Err(ToolError::Blocked {
            tool: "apply_patch",
            message: blocked.to_string(),
        });
    }
    let abs = require_absolute_path(&p).map_err(|e| ToolError::Validation {
        tool: "apply_patch",
        message: e.content,
    })?;
    Ok(abs.to_path_buf())
}

// ============================================================================
// Tool struct and rig::tool::Tool impl
// ============================================================================

/// Codex-native `apply_patch` tool.
///
/// Accepts the freeform Codex patch format and applies file operations
/// (add, update, delete) using internal async I/O helpers.
pub struct ApplyPatchTool {
    session_id: Uuid,
}

impl ApplyPatchTool {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

/// Arguments for the apply_patch tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ApplyPatchArgs {
    /// The patch text in Codex freeform format.
    pub patch: String,
}

impl rig::tool::Tool for ApplyPatchTool {
    const NAME: &'static str = "apply_patch";

    type Error = ToolError;
    type Args = ApplyPatchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "apply_patch".to_string(),
            description:
                "Apply a patch to create, update, or delete files. Uses freeform patch format \
                with '*** Begin Patch' / '*** End Patch' markers. Supports '*** Add File:', \
                '*** Update File:', and '*** Delete File:' operations."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ApplyPatchArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // HOOK-017: Run pre_tool_use hooks before execution
        if let Err(reason) = crate::pre_tool_hook::pre_tool_hook_check(
            self.session_id,
            "ApplyPatch",
            &serde_json::to_value(&args).unwrap_or_default(),
        ) {
            return Err(ToolError::Blocked {
                tool: "ApplyPatch",
                message: reason,
            });
        }

        let ops = parse_patch(&args.patch).map_err(|e| ToolError::Validation {
            tool: "apply_patch",
            message: e,
        })?;

        if ops.is_empty() {
            return Err(ToolError::Validation {
                tool: "apply_patch",
                message: "Patch contains no file operations".to_string(),
            });
        }

        let mut results: Vec<String> = Vec::new();

        for op in &ops {
            match op {
                PatchOp::Add { path, lines } => {
                    let abs = validate_patch_path(self.session_id, path)?;
                    let p = abs.to_string_lossy().to_string();
                    create_parent_dirs(&abs)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    let content = lines.join("\n") + "\n";
                    write_file_contents(&abs, &content)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    results.push(format!("Created {p}"));
                }

                PatchOp::Update { path, hunks } => {
                    let abs = validate_patch_path(self.session_id, path)?;
                    let p = abs.to_string_lossy().to_string();
                    let resolved =
                        require_file_exists(&abs, &p)
                            .await
                            .map_err(|e| ToolError::Validation {
                                tool: "apply_patch",
                                message: e.content,
                            })?;
                    // Read as bytes and run binary-guard BEFORE attempting UTF-8
                    // decode (BUG-143). A PDF/PNG target would otherwise surface
                    // only as a generic UTF-8 decode error.
                    let bytes = tokio::fs::read(&resolved)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: format!("Error reading file: {e}"),
                        })?;
                    if let Some(kind) = detect_bash_binary_output(&bytes) {
                        return Err(ToolError::Validation {
                            tool: "apply_patch",
                            message: format_file_tool_guard_message("apply_patch", kind),
                        });
                    }
                    let content = String::from_utf8(bytes).map_err(|e| ToolError::File {
                        tool: "apply_patch",
                        message: format!("Error reading file: {e}"),
                    })?;
                    let new_content =
                        apply_hunks(&content, hunks, &p).map_err(|e| ToolError::Validation {
                            tool: "apply_patch",
                            message: e,
                        })?;
                    write_file_contents(&resolved, &new_content)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: e.content,
                        })?;
                    results.push(format!("Updated {p}"));
                }

                PatchOp::Delete { path } => {
                    let abs = validate_patch_path(self.session_id, path)?;
                    let p = abs.to_string_lossy().to_string();
                    let resolved =
                        require_file_exists(&abs, &p)
                            .await
                            .map_err(|e| ToolError::Validation {
                                tool: "apply_patch",
                                message: e.content,
                            })?;
                    tokio::fs::remove_file(&resolved)
                        .await
                        .map_err(|e| ToolError::File {
                            tool: "apply_patch",
                            message: format!("Error deleting file: {e}"),
                        })?;
                    results.push(format!("Deleted {p}"));
                }
            }
        }

        Ok(results.join("\n"))
    }
}
