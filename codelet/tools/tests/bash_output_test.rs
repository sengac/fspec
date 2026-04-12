#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for BashOutput formatting.
//!
//! Extracted from bash.rs inline tests — verifies output formatting,
//! stderr marking, and result conversion.

use codelet_tools::bash_output::{BashOutput, STDERR_MARKER};

#[test]
fn test_bash_output_format_success_stdout_only() {
    let output = BashOutput {
        stdout: "hello world\n".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert_eq!(result, "hello world\n");
}

#[test]
fn test_bash_output_format_success_with_stderr() {
    let output = BashOutput {
        stdout: "output\n".to_string(),
        stderr: "warning: something\n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert!(result.contains("output"));
    assert!(result.contains("warning: something"));
    // Should NOT contain "Stderr:" label
    assert!(!result.contains("Stderr:"));
}

#[test]
fn test_bash_output_format_error_no_output() {
    let output = BashOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: Some(1),
        success: false,
    };
    let result = output.format_error();
    assert_eq!(result, "Command failed with exit code 1");
}

#[test]
fn test_bash_output_format_error_with_stderr() {
    let output = BashOutput {
        stdout: String::new(),
        stderr: "file not found\n".to_string(),
        exit_code: Some(2),
        success: false,
    };
    let result = output.format_error();
    assert!(result.contains("Command failed with exit code 2"));
    assert!(result.contains("file not found"));
    // Should NOT contain "Stderr:" label
    assert!(!result.contains("Stderr:"));
    assert!(!result.contains("Stdout:"));
}

#[test]
fn test_bash_output_format_error_with_both() {
    let output = BashOutput {
        stdout: "partial output\n".to_string(),
        stderr: "error details\n".to_string(),
        exit_code: Some(1),
        success: false,
    };
    let result = output.format_error();
    assert!(result.contains("Command failed with exit code 1"));
    assert!(result.contains("partial output"));
    assert!(result.contains("error details"));
    // Should NOT contain labels
    assert!(!result.contains("Stderr:"));
    assert!(!result.contains("Stdout:"));
}

#[test]
fn test_bash_output_into_result_success() {
    let output = BashOutput {
        stdout: "test\n".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.into_result();
    assert!(result.is_ok());
}

#[test]
fn test_bash_output_into_result_failure() {
    let output = BashOutput {
        stdout: String::new(),
        stderr: "error\n".to_string(),
        exit_code: Some(1),
        success: false,
    };
    let result = output.into_result();
    assert!(result.is_err());
}

// ========== Stderr Marker Tests ==========

#[test]
fn test_bash_output_stderr_marked_in_success() {
    let output = BashOutput {
        stdout: "stdout line\n".to_string(),
        stderr: "stderr line\n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    // Stderr should be marked with STDERR_MARKER
    assert!(result.contains("⚠stderr⚠stderr line"));
    // Stdout should NOT be marked
    assert!(result.contains("stdout line"));
    assert!(!result.contains("⚠stderr⚠stdout"));
}

#[test]
fn test_bash_output_stderr_marked_in_error() {
    let output = BashOutput {
        stdout: "stdout line\n".to_string(),
        stderr: "stderr line\n".to_string(),
        exit_code: Some(1),
        success: false,
    };
    let result = output.format_error();
    // Stderr should be marked with STDERR_MARKER
    assert!(result.contains("⚠stderr⚠stderr line"));
    // Stdout should NOT be marked
    assert!(result.contains("stdout line"));
    assert!(!result.contains("⚠stderr⚠stdout"));
}

#[test]
fn test_bash_output_stderr_only_marked() {
    // When only stderr, it should still be marked
    let output = BashOutput {
        stdout: String::new(),
        stderr: "only stderr\n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert!(result.contains("⚠stderr⚠only stderr"));
}

#[test]
fn test_bash_output_multiline_stderr_all_marked() {
    // Each line of stderr should be marked individually
    let output = BashOutput {
        stdout: "stdout\n".to_string(),
        stderr: "error line 1\nerror line 2\nerror line 3\n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert!(result.contains("⚠stderr⚠error line 1"));
    assert!(result.contains("⚠stderr⚠error line 2"));
    assert!(result.contains("⚠stderr⚠error line 3"));
    // Stdout should NOT be marked
    assert!(result.contains("stdout"));
    assert!(!result.contains("⚠stderr⚠stdout"));
}

#[test]
fn test_bash_output_no_stderr_no_marker() {
    // When no stderr, no marker should appear
    let output = BashOutput {
        stdout: "just stdout\n".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert!(!result.contains("⚠stderr⚠"));
    assert!(result.contains("just stdout"));
}

#[test]
fn test_bash_output_empty_stderr_no_marker() {
    // Whitespace-only stderr should not produce markers
    let output = BashOutput {
        stdout: "stdout\n".to_string(),
        stderr: "   \n  \n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.format_success();
    assert!(!result.contains("⚠stderr⚠"));
}

#[test]
fn test_bash_output_stderr_marker_constant() {
    // Verify the marker constant matches what TypeScript expects
    assert_eq!(STDERR_MARKER, "⚠stderr⚠");
}

#[test]
fn test_bash_output_error_with_only_stderr() {
    // Error case with only stderr should mark it
    let output = BashOutput {
        stdout: String::new(),
        stderr: "fatal error\n".to_string(),
        exit_code: Some(1),
        success: false,
    };
    let result = output.format_error();
    assert!(result.contains("Command failed with exit code 1"));
    assert!(result.contains("⚠stderr⚠fatal error"));
}

#[test]
fn test_bash_output_into_result_preserves_markers() {
    // Verify into_result() preserves stderr markers
    let output = BashOutput {
        stdout: "out\n".to_string(),
        stderr: "err\n".to_string(),
        exit_code: Some(0),
        success: true,
    };
    let result = output.into_result().unwrap();
    assert!(result.contains("⚠stderr⚠err"));
    assert!(result.contains("out"));
    assert!(!result.contains("⚠stderr⚠out"));
}
