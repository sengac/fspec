// End-to-end integration test for CLI-007 diff rendering
// Tests that Edit and Write tools actually return diff-formatted output

use codelet::tools::{EditTool, Tool, WriteTool};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_edit_tool_returns_diff_format() {
    // Create temp file
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("test.txt");
    fs::write(&test_file, "let x = 3;\nlet y = 5;\n").unwrap();

    // Execute Edit tool
    let edit_tool = EditTool::new();
    let args = json!({
        "file_path": test_file.to_str().unwrap(),
        "old_string": "let x = 3;",
        "new_string": "let x = 10;"
    });

    let result = edit_tool.execute(args).await.unwrap();

    // Verify result contains diff format
    assert!(!result.is_error, "Edit should succeed");
    let output = result.content;

    println!("Edit tool output:\n{}", output);

    // Should contain File: header
    assert!(
        output.contains("File:"),
        "Output should contain 'File:' header"
    );

    // Should contain deletion line
    assert!(
        output.contains("- let x = 3;"),
        "Output should contain deletion with '-' prefix"
    );

    // Should contain addition line
    assert!(
        output.contains("+ let x = 10;"),
        "Output should contain addition with '+' prefix"
    );
}

#[tokio::test]
async fn test_write_tool_returns_diff_format() {
    // Create temp directory
    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("new_file.txt");

    // Execute Write tool
    let write_tool = WriteTool::new();
    let content = "fn main() {\n    println!(\"hello\");\n}";
    let args = json!({
        "file_path": test_file.to_str().unwrap(),
        "content": content
    });

    let result = write_tool.execute(args).await.unwrap();

    // Verify result contains diff format
    assert!(!result.is_error, "Write should succeed");
    let output = result.content;

    println!("Write tool output:\n{}", output);

    // Should contain File: header
    assert!(
        output.contains("File:"),
        "Output should contain 'File:' header"
    );

    // Each line should be prefixed with '+'
    assert!(
        output.contains("+ fn main() {"),
        "First line should have '+' prefix"
    );
    assert!(output.contains("+ }"), "Last line should have '+' prefix");
}

#[tokio::test]
async fn test_diff_rendering_integration_with_ansi_codes() {
    use codelet::cli::diff::render_diff_line;

    // Simulate what interactive.rs does with Edit tool output
    let edit_output = "File: test.rs\n- old code\n+ new code";

    let lines: Vec<&str> = edit_output.lines().collect();
    let mut rendered_lines = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let (prefix, content) = if let Some(stripped) = line.strip_prefix('+') {
            ('+', stripped.trim())
        } else if let Some(stripped) = line.strip_prefix('-') {
            ('-', stripped.trim())
        } else if line.starts_with("File:") {
            // Keep file headers as-is
            rendered_lines.push(line.to_string());
            continue;
        } else {
            (' ', *line)
        };

        let diff_line = render_diff_line(i + 1, prefix, content);
        rendered_lines.push(diff_line);
    }

    let result = rendered_lines.join("\n");
    println!("Rendered output:\n{}", result);

    // Verify ANSI color codes are applied
    assert!(
        result.contains("\x1b[31m"),
        "Should contain red ANSI code for deletion"
    );
    assert!(
        result.contains("\x1b[32m"),
        "Should contain green ANSI code for addition"
    );
    assert!(result.contains("old code"), "Should contain old code");
    assert!(result.contains("new code"), "Should contain new code");
}
