//! Path validation utilities for tools
//!
//! Shared validation functions to ensure DRY compliance across file tools.
//! All I/O operations use tokio::fs for non-blocking async execution.

use super::ToolOutput;
use std::path::Path;

/// Validate that a file path is absolute
///
/// Returns the Path reference on success, or a ToolOutput error on failure.
/// This is a synchronous validation (no I/O involved).
pub fn require_absolute_path(file_path: &str) -> Result<&Path, ToolOutput> {
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return Err(ToolOutput::error(
            "Error: file_path must be absolute".to_string(),
        ));
    }
    Ok(path)
}

/// Validate that a file exists at the given path (async, non-blocking)
///
/// Returns Ok(()) on success, or a ToolOutput error if file not found.
pub async fn require_file_exists(path: &Path, file_path: &str) -> Result<(), ToolOutput> {
    match tokio::fs::try_exists(path).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(ToolOutput::error(format!(
            "Error: File not found: {file_path}"
        ))),
        Err(e) => Err(ToolOutput::error(format!(
            "Error checking file existence: {e}"
        ))),
    }
}

/// Read file contents with error handling (async, non-blocking)
///
/// Returns the file contents on success, or a ToolOutput error on failure.
pub async fn read_file_contents(path: &Path) -> Result<String, ToolOutput> {
    tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ToolOutput::error(format!("Error reading file: {e}")))
}

/// Write file contents with error handling (async, non-blocking)
///
/// Returns Ok(()) on success, or a ToolOutput error on failure.
pub async fn write_file_contents(path: &Path, content: &str) -> Result<(), ToolOutput> {
    tokio::fs::write(path, content)
        .await
        .map_err(|e| ToolOutput::error(format!("Error writing file: {e}")))
}

/// Create parent directories if they don't exist (async, non-blocking)
///
/// Returns Ok(()) on success, or a ToolOutput error on failure.
/// Uses create_dir_all which is idempotent (succeeds if directory already exists).
pub async fn create_parent_dirs(path: &Path) -> Result<(), ToolOutput> {
    if let Some(parent) = path.parent() {
        // Skip if parent is empty path (e.g., for "/file.txt" where parent is "/")
        if parent.as_os_str().is_empty() {
            return Ok(());
        }
        // create_dir_all is idempotent - succeeds if directory already exists
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ToolOutput::error(format!("Error creating directories: {e}")))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_require_absolute_path_valid() {
        let result = require_absolute_path("/home/user/file.txt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_require_absolute_path_relative() {
        let result = require_absolute_path("relative/path.txt");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.content.contains("must be absolute"));
            assert!(err.is_error);
        }
    }

    #[tokio::test]
    async fn test_require_file_exists_missing() {
        let path = Path::new("/nonexistent/file.txt");
        let result = require_file_exists(path, "/nonexistent/file.txt").await;
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.content.contains("File not found"));
        }
    }
}
