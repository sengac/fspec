//! Path validation utilities for tools
//!
//! Shared validation functions to ensure DRY compliance across file tools.

use super::ToolOutput;
use std::path::Path;

/// Validate that a file path is absolute
///
/// Returns the Path reference on success, or a ToolOutput error on failure.
pub fn require_absolute_path(file_path: &str) -> Result<&Path, ToolOutput> {
    let path = Path::new(file_path);
    if !path.is_absolute() {
        return Err(ToolOutput::error(
            "Error: file_path must be absolute".to_string(),
        ));
    }
    Ok(path)
}

/// Validate that a file exists at the given path
///
/// Returns Ok(()) on success, or a ToolOutput error if file not found.
pub fn require_file_exists(path: &Path, file_path: &str) -> Result<(), ToolOutput> {
    if !path.exists() {
        return Err(ToolOutput::error(format!(
            "Error: File not found: {file_path}"
        )));
    }
    Ok(())
}

/// Read file contents with error handling
///
/// Returns the file contents on success, or a ToolOutput error on failure.
pub fn read_file_contents(path: &Path) -> Result<String, ToolOutput> {
    std::fs::read_to_string(path).map_err(|e| ToolOutput::error(format!("Error reading file: {e}")))
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

    #[test]
    fn test_require_file_exists_missing() {
        let path = Path::new("/nonexistent/file.txt");
        let result = require_file_exists(path, "/nonexistent/file.txt");
        assert!(result.is_err());
        if let Err(err) = result {
            assert!(err.content.contains("File not found"));
        }
    }
}
