
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/core-file-tools-implementation.feature
//!
//! Tests for Core File Tools (Read, Write, Edit) - CORE-002

use codelet::tools::{
    edit::EditTool,
    limits::OutputLimits,
    read::ReadTool,
    truncation::{format_truncation_warning, truncate_output},
    write::WriteTool,
    Tool,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ==========================================
// READ TOOL TESTS
// ==========================================

/// Scenario: Read file returns contents with 1-based line numbers
#[tokio::test]
async fn test_read_file_returns_contents_with_line_numbers() {
    // @step Given a file exists at absolute path "/home/user/src/index.ts" with content:
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("index.ts");
    fs::write(
        &file_path,
        "import fs from 'fs';\nimport path from 'path';\n",
    )
    .unwrap();

    // @step When I execute the Read tool with file_path "/home/user/src/index.ts"
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the output should contain "1: import fs from 'fs';"
    assert!(result.content.contains("1: import fs from 'fs';"));

    // @step And the output should contain "2: import path from 'path';"
    assert!(result.content.contains("2: import path from 'path';"));
}

/// Scenario: Read file with offset and limit returns specified line range
#[tokio::test]
async fn test_read_file_with_offset_and_limit() {
    // @step Given a file exists at absolute path "/home/user/large.ts" with 200 lines
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large.ts");
    let content: String = (1..=200).map(|i| format!("line {}\n", i)).collect();
    fs::write(&file_path, content).unwrap();

    // @step When I execute the Read tool with file_path "/home/user/large.ts" offset 50 and limit 100
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "offset": 50,
            "limit": 100
        }))
        .await
        .unwrap();

    // @step Then the output should start with line number 50
    assert!(result.content.starts_with("50: "));

    // @step And the output should contain exactly 100 lines
    // Count lines that start with a number (actual content lines, not truncation warnings)
    let content_lines: Vec<&str> = result
        .content
        .lines()
        .filter(|l| {
            l.chars()
                .next()
                .map(|c| c.is_ascii_digit())
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(content_lines.len(), 100);

    // @step And the output should end with line number 149
    assert!(content_lines.last().unwrap().starts_with("149: "));
}

/// Scenario: Read file exceeding line limit is truncated with warning
#[tokio::test]
async fn test_read_file_truncated_with_warning() {
    // @step Given a file exists at absolute path "/home/user/huge.ts" with 3000 lines
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("huge.ts");
    let content: String = (1..=3000).map(|i| format!("line {}\n", i)).collect();
    fs::write(&file_path, content).unwrap();

    // @step When I execute the Read tool with file_path "/home/user/huge.ts"
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the output should contain at most 2000 lines
    let content_lines: Vec<&str> = result
        .content
        .lines()
        .filter(|l| !l.starts_with("..."))
        .collect();
    assert!(content_lines.len() <= OutputLimits::MAX_LINES);

    // @step And the output should end with a truncation warning
    assert!(result.content.contains("truncated"));
    assert!(result.truncated);

    // @step And the truncation warning should indicate the remaining line count
    assert!(result.content.contains("1000")); // 3000 - 2000 = 1000 remaining
}

/// Scenario: Read file with relative path returns error
#[tokio::test]
async fn test_read_file_relative_path_error() {
    // @step When I execute the Read tool with file_path "src/main.rs"
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": "src/main.rs"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: file_path must be absolute"
    assert!(result.content.contains("Error: file_path must be absolute"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);
}

/// Scenario: Read file truncates long lines with ellipsis
#[tokio::test]
async fn test_read_file_truncates_long_lines() {
    // @step Given a file exists at absolute path "/home/user/wide.ts" with a line exceeding 2000 characters
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("wide.ts");
    let long_line = "x".repeat(3000);
    fs::write(&file_path, &long_line).unwrap();

    // @step When I execute the Read tool with file_path "/home/user/wide.ts"
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then lines exceeding 2000 characters should be truncated
    let first_line = result.content.lines().next().unwrap();
    // Line format is "N: content..." so check content length
    assert!(first_line.len() <= OutputLimits::MAX_LINE_LENGTH + 10); // Allow for line number prefix

    // @step And truncated lines should end with "..."
    assert!(first_line.ends_with("..."));
}

/// Scenario: Read non-existent file returns error
#[tokio::test]
async fn test_read_nonexistent_file_error() {
    // @step When I execute the Read tool with file_path "/home/user/nonexistent.ts"
    let tool = ReadTool::new();
    let result = tool
        .execute(json!({
            "file_path": "/home/user/nonexistent.ts"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: File not found"
    assert!(result.content.contains("Error: File not found"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);
}

// ==========================================
// WRITE TOOL TESTS
// ==========================================

/// Scenario: Write tool creates new file successfully
#[tokio::test]
async fn test_write_tool_creates_new_file() {
    // @step Given the file "/home/user/new.ts" does not exist
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("new.ts");
    assert!(!file_path.exists());

    // @step When I execute the Write tool with file_path "/home/user/new.ts" and content "export const foo = 1;"
    let tool = WriteTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "content": "export const foo = 1;"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Successfully wrote to /home/user/new.ts"
    assert!(result.content.contains("Successfully wrote to"));

    // @step And the file "/home/user/new.ts" should exist with the written content
    assert!(file_path.exists());
    let written_content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(written_content, "export const foo = 1;");
}

/// Scenario: Write tool overwrites existing file
#[tokio::test]
async fn test_write_tool_overwrites_existing_file() {
    // @step Given a file exists at absolute path "/home/user/old.ts" with content "old content"
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("old.ts");
    fs::write(&file_path, "old content").unwrap();

    // @step When I execute the Write tool with file_path "/home/user/old.ts" and content "new content"
    let tool = WriteTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "content": "new content"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Successfully wrote to /home/user/old.ts"
    assert!(result.content.contains("Successfully wrote to"));

    // @step And the file should contain "new content"
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("new content"));

    // @step And the file should not contain "old content"
    assert!(!content.contains("old content"));
}

/// Scenario: Write tool creates parent directories if missing
#[tokio::test]
async fn test_write_tool_creates_parent_directories() {
    // @step Given the directory "/home/user/nested/deep/" does not exist
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("nested").join("deep").join("file.ts");
    assert!(!file_path.parent().unwrap().exists());

    // @step When I execute the Write tool with file_path "/home/user/nested/deep/file.ts" and content "content"
    let tool = WriteTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "content": "content"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Successfully wrote"
    assert!(result.content.contains("Successfully wrote"));

    // @step And the file "/home/user/nested/deep/file.ts" should exist
    assert!(file_path.exists());
}

/// Scenario: Write tool with relative path returns error
#[tokio::test]
async fn test_write_tool_relative_path_error() {
    // @step When I execute the Write tool with file_path "relative/path.ts" and content "content"
    let tool = WriteTool::new();
    let result = tool
        .execute(json!({
            "file_path": "relative/path.ts",
            "content": "content"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: file_path must be absolute"
    assert!(result.content.contains("Error: file_path must be absolute"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);
}

// ==========================================
// EDIT TOOL TESTS
// ==========================================

/// Scenario: Edit tool replaces first occurrence only
#[tokio::test]
async fn test_edit_tool_replaces_first_occurrence() {
    // @step Given a file exists at absolute path "/home/user/main.rs" with content:
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("main.rs");
    fs::write(&file_path, "let foo = 1;\nlet foo = 2;\n").unwrap();

    // @step When I execute the Edit tool with file_path "/home/user/main.rs" old_string "foo" and new_string "bar"
    let tool = EditTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "old_string": "foo",
            "new_string": "bar"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Successfully edited"
    assert!(result.content.contains("Successfully edited"));

    // @step And the file should contain "let bar = 1;"
    let content = fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("let bar = 1;"));

    // @step And the file should contain "let foo = 2;"
    assert!(content.contains("let foo = 2;"));
}

/// Scenario: Edit tool returns error when old_string not found
#[tokio::test]
async fn test_edit_tool_old_string_not_found() {
    // @step Given a file exists at absolute path "/home/user/test.rs" with content "hello world"
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, "hello world").unwrap();

    // @step When I execute the Edit tool with file_path "/home/user/test.rs" old_string "xyz123" and new_string "replacement"
    let tool = EditTool::new();
    let result = tool
        .execute(json!({
            "file_path": file_path.to_string_lossy(),
            "old_string": "xyz123",
            "new_string": "replacement"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: old_string not found in file"
    assert!(result
        .content
        .contains("Error: old_string not found in file"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);

    // @step And the file content should be unchanged
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "hello world");
}

/// Scenario: Edit tool with relative path returns error
#[tokio::test]
async fn test_edit_tool_relative_path_error() {
    // @step When I execute the Edit tool with file_path "relative.ts" old_string "a" and new_string "b"
    let tool = EditTool::new();
    let result = tool
        .execute(json!({
            "file_path": "relative.ts",
            "old_string": "a",
            "new_string": "b"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: file_path must be absolute"
    assert!(result.content.contains("Error: file_path must be absolute"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);
}

/// Scenario: Edit non-existent file returns error
#[tokio::test]
async fn test_edit_nonexistent_file_error() {
    // @step When I execute the Edit tool with file_path "/home/user/missing.ts" old_string "a" and new_string "b"
    let tool = EditTool::new();
    let result = tool
        .execute(json!({
            "file_path": "/home/user/missing.ts",
            "old_string": "a",
            "new_string": "b"
        }))
        .await
        .unwrap();

    // @step Then the output should contain "Error: File not found"
    assert!(result.content.contains("Error: File not found"));

    // @step And the tool execution should indicate an error
    assert!(result.is_error);
}

// ==========================================
// TRUNCATION UTILITY TESTS
// ==========================================

#[test]
fn test_truncate_output_under_limit() {
    let lines = vec!["line 1".to_string(), "line 2".to_string()];
    let result = truncate_output(&lines, 1000);
    assert!(!result.char_truncated);
    assert_eq!(result.remaining_count, 0);
    assert_eq!(result.included_count, 2);
}

#[test]
fn test_truncate_output_over_limit() {
    let lines: Vec<String> = (1..=100).map(|i| format!("line {}", i)).collect();
    let result = truncate_output(&lines, 100);
    assert!(result.char_truncated);
    assert!(result.remaining_count > 0);
    assert!(result.included_count < 100);
}

#[test]
fn test_format_truncation_warning() {
    let warning = format_truncation_warning(50, "lines", true, 30000);
    assert!(warning.contains("50"));
    assert!(warning.contains("lines"));
    assert!(warning.contains("truncated"));
}

#[test]
fn test_output_limits_constants() {
    assert_eq!(OutputLimits::MAX_OUTPUT_CHARS, 30000);
    assert_eq!(OutputLimits::MAX_LINE_LENGTH, 2000);
    assert_eq!(OutputLimits::MAX_LINES, 2000);
}

// ==========================================
// TOOL REGISTRY WIRING TESTS
// ==========================================

use codelet::tools::ToolRegistry;

/// Scenario: ToolRegistry default includes all core tools
#[test]
fn test_tool_registry_default_has_core_tools() {
    // @step When I create a default ToolRegistry
    let registry = ToolRegistry::default();

    // @step Then it should have 7 tools registered (AstGrep, Bash, Read, Write, Edit, Grep, Glob)
    assert_eq!(registry.len(), 7);

    // @step And it should have the Bash tool
    assert!(registry.get("Bash").is_some());

    // @step And it should have the Read tool
    assert!(registry.get("Read").is_some());

    // @step And it should have the Write tool
    assert!(registry.get("Write").is_some());

    // @step And it should have the Edit tool
    assert!(registry.get("Edit").is_some());
}

/// Scenario: ToolRegistry can execute tools by name
#[tokio::test]
async fn test_tool_registry_execute_by_name() {
    // @step Given a ToolRegistry with core tools
    let registry = ToolRegistry::default();
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");

    // @step When I execute the Write tool by name
    let result = registry
        .execute(
            "Write",
            json!({
                "file_path": file_path.to_string_lossy(),
                "content": "test content"
            }),
        )
        .await
        .unwrap();

    // @step Then the execution should succeed
    assert!(!result.is_error);
    assert!(result.content.contains("Successfully wrote"));

    // @step And when I execute the Read tool by name
    let result = registry
        .execute(
            "Read",
            json!({
                "file_path": file_path.to_string_lossy()
            }),
        )
        .await
        .unwrap();

    // @step Then I should get the file contents
    assert!(!result.is_error);
    assert!(result.content.contains("test content"));
}

/// Scenario: ToolRegistry returns error for unknown tool
#[tokio::test]
async fn test_tool_registry_unknown_tool_error() {
    // @step Given a ToolRegistry
    let registry = ToolRegistry::default();

    // @step When I try to execute an unknown tool
    let result = registry.execute("UnknownTool", json!({})).await.unwrap();

    // @step Then the result should be an error
    assert!(result.is_error);
    assert!(result.content.contains("Unknown tool"));
}

// ==========================================
// RUNNER INTEGRATION TESTS
// ==========================================

use codelet::agent::Runner;

/// Scenario: Runner has tools wired by default
#[test]
fn test_runner_has_tools_wired() {
    // @step When I create a new Runner
    let runner = Runner::new();

    // @step Then it should have 7 tools available (AstGrep, Bash, Read, Write, Edit, Grep, Glob)
    let tools = runner.available_tools();
    assert_eq!(tools.len(), 7);

    // @step And the tools should be accessible
    assert!(runner.tools().get("Bash").is_some());
    assert!(runner.tools().get("Read").is_some());
    assert!(runner.tools().get("Write").is_some());
    assert!(runner.tools().get("Edit").is_some());
}

/// Scenario: Runner can execute tools
#[tokio::test]
async fn test_runner_execute_tool() {
    // @step Given a Runner with default tools
    let runner = Runner::new();
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("runner_test.txt");

    // @step When I execute a tool through the runner
    let result = runner
        .execute_tool(
            "Write",
            json!({
                "file_path": file_path.to_string_lossy(),
                "content": "runner test"
            }),
        )
        .await
        .unwrap();

    // @step Then the tool execution should succeed
    assert!(!result.is_error);

    // @step And the file should exist with correct content
    let content = fs::read_to_string(&file_path).unwrap();
    assert_eq!(content, "runner test");
}
