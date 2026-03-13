//! Codex `view_image` tool implementation.
//!
//! Views local image files and returns base64-encoded image data.
//! This is a standalone `rig::tool::Tool` because `view_image` has no
//! equivalent in other providers (Codex-native only).
//!
//! Delegates to the same image validation logic used by `ReadTool`:
//! - File type detection via `detect_file_type`
//! - Base64 size validation (5MB limit)
//! - Pixel dimension validation
//!
//! Feature: spec/features/codex-view-image.feature

use super::blocklist::check_file_path;
use super::error::ToolError;
use super::facade::validate_and_resolve_path;
use super::file_type::{detect_file_type, FileType, ImageMediaType};
use super::read::validate_and_encode_image;
use super::validation::{require_absolute_path, require_file_exists};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::fs;
use uuid::Uuid;

/// Codex-native `view_image` tool.
///
/// Views a local image file and returns base64-encoded image data with
/// media type. Only accepts binary image formats (PNG, JPEG, GIF, WEBP).
/// SVG, PDF, text, and other file types are rejected.
pub struct ViewImageTool {
    session_id: Uuid,
}

impl ViewImageTool {
    pub fn new(session_id: Uuid) -> Self {
        Self { session_id }
    }
}

/// Arguments for the view_image tool.
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct ViewImageArgs {
    /// Local filesystem path to an image file
    pub path: String,
}

impl rig::tool::Tool for ViewImageTool {
    const NAME: &'static str = "view_image";

    type Error = ToolError;
    type Args = ViewImageArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> rig::completion::ToolDefinition {
        rig::completion::ToolDefinition {
            name: "view_image".to_string(),
            description:
                "View a local image from the filesystem (only use if given a full filepath \
                by the user, and the image isn't already attached to the thread context \
                within <image ...> tags)."
                    .to_string(),
            parameters: serde_json::to_value(schemars::schema_for!(ViewImageArgs))
                .unwrap_or_else(|_| json!({"type": "object"})),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // 1. Validate and resolve path (handles worktree isolation)
        let resolved_path =
            validate_and_resolve_path(self.session_id, &args.path, "view_image")?;
        let file_path_str = resolved_path.to_string_lossy().to_string();

        // 2. Check file path against blocklist before any I/O
        if let Err(blocked) = check_file_path(&file_path_str) {
            return Err(ToolError::Blocked {
                tool: "view_image",
                message: blocked.to_string(),
            });
        }

        // 3. Validate absolute path
        let path =
            require_absolute_path(&file_path_str).map_err(|e| ToolError::Validation {
                tool: "view_image",
                message: e.content,
            })?;

        // 4. Check file exists
        require_file_exists(path, &file_path_str)
            .await
            .map_err(|e| ToolError::Validation {
                tool: "view_image",
                message: e.content,
            })?;

        // 5. Read file as binary
        let binary_content =
            fs::read(path)
                .await
                .map_err(|e| ToolError::File {
                    tool: "view_image",
                    message: format!("Error reading file: {e}"),
                })?;

        // 6. Detect file type and reject non-binary-image files
        let file_type = detect_file_type(path, &binary_content);
        let media_type = match file_type {
            FileType::Image(ImageMediaType::Svg) => {
                return Err(ToolError::Validation {
                    tool: "view_image",
                    message: format!(
                        "File is not a supported image format for view_image: {file_path_str}\n\
                         SVG files are text-based XML and should be read with the Read tool instead."
                    ),
                });
            }
            FileType::Image(media_type) => media_type,
            _ => {
                return Err(ToolError::Validation {
                    tool: "view_image",
                    message: format!(
                        "File is not a supported image format for view_image: {file_path_str}\n\
                         Supported formats: PNG, JPEG, GIF, WEBP.\n\
                         Use the Read tool to view text files, PDFs, or other document types."
                    ),
                });
            }
        };

        // 7. Validate size + dimensions and encode (shared with ReadTool)
        let output = validate_and_encode_image(
            &binary_content,
            media_type,
            &file_path_str,
            "view_image",
        )?;

        serde_json::to_string(&output).map_err(|e| ToolError::File {
            tool: "view_image",
            message: format!("Error serializing output: {e}"),
        })
    }
}
