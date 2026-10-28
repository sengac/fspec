mod tests {
    use super::*;

    // ========== JSON-based error detection tests ==========

    #[test]
    fn test_detect_tool_error_success_true() {
        // Tool facade returns success: true - should NOT be an error
        let result = r#"{"success":true,"content":"file contents here"}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_success_false() {
        // Tool facade returns success: false - IS an error
        let result = r#"{"success":false,"error":"File not found"}"#;
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_error_field() {
        // Error field present with non-empty value - IS an error
        let result = r#"{"error":"Something went wrong"}"#;
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_null_error() {
        // Error field present but null - NOT an error
        let result = r#"{"success":true,"error":null}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_empty_error() {
        // Error field present but empty string - NOT an error
        let result = r#"{"success":true,"error":""}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_json_content_with_error_word() {
        // JSON with content containing "Error:" but success: true
        let result = r#"{"success":true,"content":"Error: this is just example text"}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_json_without_success_or_error() {
        // JSON without success or error fields - NOT an error
        let result = r#"{"data":"some value","count":42}"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_nested_json() {
        // Nested JSON - only top-level success/error counts
        let result = r#"{"success":true,"data":{"error":"nested error doesn't count"}}"#;
        assert!(!detect_tool_error(result));
    }

    // ========== Plain-text error pattern detection tests ==========

    #[test]
    fn test_detect_tool_error_bash_exit_code() {
        // Bash command failure - IS an error
        let result = "Command failed with exit code 1\nerror: unknown command 'bootstrap2'\n(Did you mean bootstrap?)";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_timeout() {
        // Timeout error - IS an error
        let result = "Operation timed out after 30 seconds";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_token_limit() {
        // Token limit error - IS an error
        let result = "Token limit exceeded: /path/to/file has ~50000 tokens (limit: 25000)";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_invalid_pattern() {
        // Pattern error - IS an error
        let result = "Invalid pattern: [unclosed bracket";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_unsupported_language() {
        // Language error - IS an error
        let result = "Unsupported language: brainfuck";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_not_found() {
        // Tool not found error - IS an error
        let result = "Tool not found: nonexistent_tool";
        assert!(detect_tool_error(result));
    }

    // ========== False positive prevention tests ==========

    #[test]
    fn test_detect_tool_error_plain_text_with_error_word() {
        // Plain text containing "Error:" - should NOT be marked as error
        // This was the bug: file content containing "error:" was falsely flagged
        let result = r#"1: # Agent Development Guidelines
2: This document describes error handling.
3: Use ToolError for tool failures.
4: Example: console.error('Error: something failed');"#;
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_non_json_normal_output() {
        // Normal plain text output - NOT an error
        let result = "This is plain text output from a tool";
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_command_in_middle() {
        // Error pattern appearing in middle of content - NOT an error
        // Only patterns at START of message indicate actual errors
        let result = "The message 'Command failed with exit code 1' was logged";
        assert!(!detect_tool_error(result));
    }

    // ========== Legacy error wrapper pattern tests ==========

    #[test]
    fn test_detect_tool_error_toolset_error_wrapper() {
        // Legacy ToolServerError wrapper - IS an error
        let result = "Toolset error: Command failed with exit code 1\nerror: unknown command";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_call_error_wrapper() {
        // Legacy ToolSetError wrapper - IS an error
        let result = "ToolCallError: File not found: /path/to/file";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_tool_server_error_wrapper() {
        // Legacy RequestError wrapper - IS an error
        let result = "ToolServerError: Connection refused";
        assert!(detect_tool_error(result));
    }

    // ========== Stderr marker tests (integration with bash output) ==========

    #[test]
    fn test_detect_tool_error_with_stderr_markers_success() {
        // Successful command with stderr markers should NOT be flagged as error
        // (The markers are for UI coloring, not error status)
        let result = "stdout output\n⚠stderr⚠warning: deprecated function";
        assert!(!detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_with_stderr_markers_and_error() {
        // Failed command has error prefix + stderr markers
        let result = "Command failed with exit code 1\n⚠stderr⚠error: file not found";
        assert!(detect_tool_error(result));
    }

    #[test]
    fn test_detect_tool_error_stderr_marker_only() {
        // Only stderr markers (no actual error) should NOT be flagged
        let result = "⚠stderr⚠this is just a warning";
        assert!(!detect_tool_error(result));
    }
}
