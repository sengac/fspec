//! BUG-431: read a card attachment from its workspace on the daemon, and map
//! it to the workspace-relative path a client caches it under.

use std::path::{Path, PathBuf};

use crate::handlers::validate_path;

/// The largest attachment served to a client.
pub const MAX_ATTACHMENT_BYTES: u64 = 64 * 1024 * 1024;

/// The bytes of attachment `path` (as stored on the card) inside
/// `workspace`. Every refusal is an error with the reason shown to the user.
pub fn read_workspace_attachment(workspace: &Path, path: &str) -> Result<Vec<u8>, String> {
    let full = validate_path(workspace, path)
        .ok_or_else(|| "attachment path rejected: it escapes the card workspace".to_string())?;
    let metadata = std::fs::metadata(&full).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            "attachment not found (missing file in the card workspace)".to_string()
        } else {
            format!("attachment cannot be read: {error}")
        }
    })?;
    if metadata.is_dir() {
        return Err("attachment is a directory, not a viewable file".to_string());
    }
    if !metadata.is_file() {
        return Err("attachment is not a regular file".to_string());
    }
    if metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err(format!(
            "attachment is too large to open ({} bytes, limit {MAX_ATTACHMENT_BYTES})",
            metadata.len()
        ));
    }
    std::fs::read(&full).map_err(|error| format!("attachment cannot be read: {error}"))
}

/// `path` relative to `workspace` (lexically; neither needs to exist on this
/// machine), or `None` when it escapes the workspace.
pub fn workspace_relative(workspace: &Path, path: &str) -> Option<PathBuf> {
    let full = validate_path(workspace, path)?;
    let root = validate_path(workspace, ".")?;
    full.strip_prefix(&root).ok().map(Path::to_path_buf)
}
