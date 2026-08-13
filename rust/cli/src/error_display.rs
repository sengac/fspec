//! Error display utilities
//!
//! Provides clean, user-friendly error message formatting for CLI display.
//! Single Responsibility: Transform technical error messages into user-readable output.

use crate::terminal;

// ============================================================================
// Exit Code Extraction
// ============================================================================

/// Extract exit code from an error message if present.
///
/// Looks for patterns like "exit code 1" or "exit code 127".
///
/// # Example
/// ```
/// use codelet_cli::error_display::extract_exit_code;
///
/// assert_eq!(extract_exit_code("Command failed with exit code 1"), Some(1));
/// assert_eq!(extract_exit_code("Something else"), None);
/// ```
pub fn extract_exit_code(error: &str) -> Option<i32> {
    // Look for "exit code N" pattern
    if let Some(idx) = error.find("exit code ") {
        let after = &error[idx + 10..];
        // Extract consecutive digits
        let code_str: String = after.chars().take_while(char::is_ascii_digit).collect();
        code_str.parse().ok()
    } else {
        None
    }
}

// ============================================================================
// Error Formatting
// ============================================================================

/// Format a tool execution error for user-friendly CLI display.
///
/// This function:
/// 1. Extracts exit code if present (for command failures)
/// 2. Formats with appropriate coloring and symbols
///
/// # Output Format
///
/// For command failures with exit code:
/// ```text
/// ✗ Command exited with code 1
/// error: unknown command 'foo'
/// ```
///
/// For other errors:
/// ```text
/// File not found: /path/to/file
/// ```
pub fn format_tool_error(error: &str) -> String {
    // Check if this is a command failure with exit code
    if let Some(exit_code) = extract_exit_code(error) {
        format_command_failure(exit_code, error)
    } else {
        // Generic error - just show in red
        terminal::bold_red(error)
    }
}

/// Format a command failure with exit code.
///
/// Separates the exit code header from the command output.
/// Colors the entire output red.
fn format_command_failure(exit_code: i32, error: &str) -> String {
    // Find where the actual output starts (after "Command failed with exit code N\n")
    let header = format!("✗ Command exited with code {exit_code}");

    if let Some(newline_idx) = error.find('\n') {
        let output = error[newline_idx + 1..].trim();
        if output.is_empty() {
            terminal::bold_red(&header)
        } else {
            // Color the entire thing red (header + output)
            terminal::bold_red(&format!("{header}\n{output}"))
        }
    } else {
        // No output after the exit code message
        terminal::bold_red(&header)
    }
}

/// Format a general CLI error (not tool-specific).
///
/// Used for API errors, configuration errors, etc.
pub fn format_cli_error(error: &str) -> String {
    terminal::bold_red(&format!("Error: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== extract_exit_code tests ==========

    #[test]
    fn test_extract_exit_code_simple() {
        assert_eq!(
            extract_exit_code("Command failed with exit code 1"),
            Some(1)
        );
    }

    #[test]
    fn test_extract_exit_code_large() {
        assert_eq!(
            extract_exit_code("exit code 127 - command not found"),
            Some(127)
        );
    }

    #[test]
    fn test_extract_exit_code_missing() {
        assert_eq!(extract_exit_code("Something else"), None);
    }

    #[test]
    fn test_extract_exit_code_zero() {
        assert_eq!(extract_exit_code("exit code 0"), Some(0));
    }

    // ========== format_tool_error tests ==========

    #[test]
    fn test_format_tool_error_with_output() {
        let error = "Command failed with exit code 1\nerror: unknown command";
        let formatted = format_tool_error(error);
        assert!(formatted.contains("✗ Command exited with code 1"));
        assert!(formatted.contains("unknown command"));
        // Should be colored red
        assert!(formatted.contains(terminal::style::RED));
    }

    #[test]
    fn test_format_tool_error_no_output() {
        let error = "Command failed with exit code 2";
        let formatted = format_tool_error(error);
        assert!(formatted.contains("✗ Command exited with code 2"));
        assert!(formatted.contains(terminal::style::RED));
    }

    #[test]
    fn test_format_tool_error_non_command() {
        let error = "File not found: /path/to/file";
        let formatted = format_tool_error(error);
        assert!(formatted.contains("File not found"));
        assert!(formatted.contains(terminal::style::RED));
    }

    // ========== format_cli_error tests ==========

    #[test]
    fn test_format_cli_error_adds_prefix() {
        let error = "API request failed";
        let formatted = format_cli_error(error);
        assert!(formatted.contains("Error:"));
        assert!(formatted.contains("API request failed"));
        assert!(formatted.contains(terminal::style::RED));
    }
}
