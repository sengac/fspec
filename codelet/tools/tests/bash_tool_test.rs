//! Feature: spec/features/bash-tool-implementation.feature
//!
//! Tests for Bash Tool Implementation - CORE-003

use codelet_tools::{bash::BashTool, limits::OutputLimits, Tool, ToolRegistry};
use serde_json::json;
use std::time::Duration;

// ==========================================
// BASH TOOL EXECUTION TESTS
// ==========================================

/// Scenario: Execute simple command successfully
#[tokio::test]
async fn test_execute_simple_command_successfully() {
    // @step Given the Bash tool is available
    let tool = BashTool::new();

    // @step When I execute the Bash tool with command "echo hello"
    let result = tool
        .execute(json!({
            "command": "echo hello"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "hello"
    assert!(result.content.contains("hello"));

    // @step And the result should not be an error
    assert!(!result.is_error);
}

/// Scenario: Execute command that fails returns error with stderr
#[tokio::test]
async fn test_execute_command_fails_returns_error() {
    // @step Given the Bash tool is available
    let tool = BashTool::new();

    // @step When I execute the Bash tool with command "ls /nonexistent_directory_12345"
    let result = tool
        .execute(json!({
            "command": "ls /nonexistent_directory_12345"
        }))
        .await
        .unwrap();

    // @step Then the result should be an error
    assert!(result.is_error);

    // @step And the output should contain error information
    assert!(
        result.content.contains("Error")
            || result.content.contains("No such file")
            || result.content.contains("cannot access")
    );
}

/// Scenario: Long output is truncated at character limit
#[tokio::test]
async fn test_long_output_truncated_at_character_limit() {
    // @step Given the Bash tool is available
    let tool = BashTool::new();

    // @step When I execute the Bash tool with a command that generates over 30000 characters
    // Generate ~50000 characters (each line is ~11 chars "line NNNNN\n")
    let result = tool
        .execute(json!({
            "command": "seq 1 5000 | while read i; do echo \"line $i\"; done"
        }))
        .await
        .unwrap();

    // @step Then the output should be truncated to at most 30000 characters
    assert!(result.content.len() <= OutputLimits::MAX_OUTPUT_CHARS + 100); // Allow for truncation message

    // @step And the output should contain a truncation warning
    assert!(result.content.contains("truncated"));
    assert!(result.truncated);
}

/// Scenario: Long lines are replaced with omission message
#[tokio::test]
async fn test_long_lines_replaced_with_omission_message() {
    // @step Given the Bash tool is available
    let tool = BashTool::new();

    // @step When I execute the Bash tool with a command that outputs a line over 2000 characters
    // Generate a single line with 3000 'x' characters
    let result = tool
        .execute(json!({
            "command": "printf '%0.sx' $(seq 1 3000)"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "[Omitted long line]"
    assert!(result.content.contains("[Omitted long line]"));
}

/// Scenario: Command timeout returns error
#[tokio::test]
async fn test_command_timeout_returns_error() {
    // @step Given the Bash tool is available with a 1 second timeout
    let tool = BashTool::with_timeout(Duration::from_secs(1));

    // @step When I execute the Bash tool with command "sleep 10"
    let result = tool
        .execute(json!({
            "command": "sleep 10"
        }))
        .await
        .unwrap();

    // @step Then the result should be an error
    assert!(result.is_error);

    // @step And the output should indicate a timeout occurred
    assert!(
        result.content.contains("timeout")
            || result.content.contains("Timeout")
            || result.content.contains("timed out")
    );
}

// ==========================================
// TOOL REGISTRY INTEGRATION TESTS
// ==========================================

/// Scenario: BashTool is registered in default ToolRegistry
#[test]
fn test_bash_tool_registered_in_default_registry() {
    // @step Given a default ToolRegistry
    let registry = ToolRegistry::default();

    // @step Then the registry should contain the "Bash" tool
    assert!(registry.get("Bash").is_some());

    // @step And the Bash tool should have the correct name and description
    let tool = registry.get("Bash").unwrap();
    assert_eq!(tool.name(), "Bash");
    assert!(!tool.description().is_empty());
}

/// Scenario: ToolRegistry can execute Bash tool
#[tokio::test]
async fn test_registry_can_execute_bash_tool() {
    // @step Given a ToolRegistry with default tools
    let registry = ToolRegistry::default();

    // @step When I execute the Bash tool through the registry with command "pwd"
    let result = registry
        .execute(
            "Bash",
            json!({
                "command": "pwd"
            }),
        )
        .await
        .unwrap();

    // @step Then the output should contain the current working directory
    // pwd returns an absolute path starting with /
    assert!(result.content.starts_with('/') || result.content.contains('/'));

    // @step And the result should not be an error
    assert!(!result.is_error);
}
