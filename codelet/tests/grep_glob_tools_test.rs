
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/grep-and-glob-tools-implementation.feature
//!
//! Tests for Grep and Glob Tools Implementation - CORE-004
//!
//! These tests verify the implementation of grep crate-based content search
//! and ignore crate-based gitignore-aware file pattern matching.

use codelet::agent::Runner;
use codelet::tools::{glob::GlobTool, grep::GrepTool, Tool, ToolRegistry};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ==========================================
// GREP TOOL EXECUTION TESTS
// ==========================================

/// Scenario: Grep search returns file paths containing pattern
#[tokio::test]
async fn test_grep_search_returns_file_paths_containing_pattern() {
    // @step Given a directory with files containing "TODO" comments
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("file1.rs");
    let file2 = temp_dir.path().join("file2.rs");
    let file3 = temp_dir.path().join("no_match.rs");
    fs::write(&file1, "// TODO: implement this\nfn foo() {}").unwrap();
    fs::write(&file2, "// TODO: fix bug\nfn bar() {}").unwrap();
    fs::write(&file3, "// This file has no match\nfn baz() {}").unwrap();

    // @step When I execute the Grep tool with pattern "TODO"
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "TODO",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain file paths of matching files
    assert!(result.content.contains("file1.rs"));
    assert!(result.content.contains("file2.rs"));

    // @step And the result should not be an error
    assert!(!result.is_error);

    // @step And the result should not contain non-matching files
    assert!(!result.content.contains("no_match.rs"));
}

/// Scenario: Grep with content mode shows matching lines with line numbers
#[tokio::test]
async fn test_grep_content_mode_shows_lines_with_numbers() {
    // @step Given a directory with source files
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("lib.rs");
    fs::write(
        &file,
        "use std::io;\n\nexport function hello() {\n    println!(\"Hello\");\n}\n",
    )
    .unwrap();

    // @step When I execute the Grep tool with pattern "export function" and output_mode "content"
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "export function",
            "path": temp_dir.path().to_string_lossy(),
            "output_mode": "content"
        }))
        .await
        .unwrap();

    // @step Then the result should contain lines with line numbers in "N:" format
    // Content mode shows file:line:content format
    assert!(result.content.contains(":3:") || result.content.contains("3:"));

    // @step And the result should contain "function"
    assert!(result.content.contains("function"));
}

/// Scenario: Grep with glob filter only searches matching files
#[tokio::test]
async fn test_grep_glob_filter_only_searches_matching_files() {
    // @step Given a directory with .ts and .js files containing "TODO"
    let temp_dir = TempDir::new().unwrap();
    let ts_file = temp_dir.path().join("app.ts");
    let js_file = temp_dir.path().join("util.js");
    fs::write(&ts_file, "// TODO: TypeScript file").unwrap();
    fs::write(&js_file, "// TODO: JavaScript file").unwrap();

    // @step When I execute the Grep tool with pattern "TODO" and glob "*.ts"
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "TODO",
            "path": temp_dir.path().to_string_lossy(),
            "glob": "*.ts"
        }))
        .await
        .unwrap();

    // @step Then the result should only contain .ts files
    assert!(result.content.contains("app.ts"));

    // @step And the result should not contain .js files
    assert!(!result.content.contains("util.js"));
}

/// Scenario: Grep with context lines includes surrounding lines
#[tokio::test]
async fn test_grep_context_lines_includes_surrounding_lines() {
    // @step Given a directory with source files
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("context.rs");
    fs::write(&file, "line 1\nline 2\n// TODO: fix this\nline 4\nline 5\n").unwrap();

    // @step When I execute the Grep tool with pattern "TODO" and -A set to 2 in content mode
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "TODO",
            "path": temp_dir.path().to_string_lossy(),
            "output_mode": "content",
            "-A": 2
        }))
        .await
        .unwrap();

    // @step Then the result should include context lines after each match
    // Should include the TODO line and at least "line 4" or "line 5" as context
    assert!(result.content.contains("TODO"));
    assert!(result.content.contains("line 4") || result.content.contains("line 5"));
}

/// Scenario: Grep with case-insensitive flag matches all cases
#[tokio::test]
async fn test_grep_case_insensitive_matches_all_cases() {
    // @step Given a file containing "ERROR", "error", and "Error"
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("logs.txt");
    fs::write(&file, "ERROR: critical\nerror: warning\nError: info\n").unwrap();

    // @step When I execute the Grep tool with pattern "error" and -i flag
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "error",
            "path": temp_dir.path().to_string_lossy(),
            "output_mode": "content",
            "-i": true
        }))
        .await
        .unwrap();

    // @step Then the result should contain all three variations
    assert!(result.content.contains("ERROR"));
    assert!(result.content.contains("error"));
    assert!(result.content.contains("Error"));
}

/// Scenario: Grep with multiline mode matches patterns spanning lines
#[tokio::test]
async fn test_grep_multiline_matches_spanning_lines() {
    // @step Given a file with a multi-line function definition
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("multiline.rs");
    fs::write(
        &file,
        "fn foo(\n    param1: i32,\n    param2: String\n) -> bool {\n    true\n}\n",
    )
    .unwrap();

    // @step When I execute the Grep tool with a multiline pattern and multiline enabled
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "fn foo\\([\\s\\S]*?\\) -> bool",
            "path": temp_dir.path().to_string_lossy(),
            "output_mode": "content",
            "multiline": true
        }))
        .await
        .unwrap();

    // @step Then the result should match content spanning multiple lines
    // Should match the function definition that spans multiple lines
    assert!(result.content.contains("fn foo") || result.content.contains("param1"));
    assert!(!result.is_error);
}

/// Scenario: Grep with count mode returns match counts per file
#[tokio::test]
async fn test_grep_count_mode_returns_match_counts() {
    // @step Given multiple files containing the search pattern
    let temp_dir = TempDir::new().unwrap();
    let file1 = temp_dir.path().join("many.rs");
    let file2 = temp_dir.path().join("few.rs");
    fs::write(&file1, "TODO one\nTODO two\nTODO three\n").unwrap();
    fs::write(&file2, "TODO single\n").unwrap();

    // @step When I execute the Grep tool with output_mode "count"
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "TODO",
            "path": temp_dir.path().to_string_lossy(),
            "output_mode": "count"
        }))
        .await
        .unwrap();

    // @step Then the result should show file paths with their match counts in "file:N" format
    // many.rs should have 3 matches, few.rs should have 1 match
    assert!(result.content.contains("many.rs") && result.content.contains("3"));
    assert!(result.content.contains("few.rs") && result.content.contains("1"));
}

/// Scenario: Grep for non-existent pattern returns no matches message
#[tokio::test]
async fn test_grep_nonexistent_pattern_returns_no_matches() {
    // @step Given a directory with source files
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("code.rs");
    fs::write(&file, "fn main() {}\n").unwrap();

    // @step When I execute the Grep tool with a non-existent pattern
    let tool = GrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "ZZZZNONEXISTENT12345",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should be "No matches found"
    assert!(result.content.contains("No matches found"));
}

// ==========================================
// GLOB TOOL EXECUTION TESTS
// ==========================================

/// Scenario: Glob search returns all files matching pattern
#[tokio::test]
async fn test_glob_search_returns_matching_files() {
    // @step Given a project directory with TypeScript files in various directories
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let nested_dir = src_dir.join("nested");
    fs::create_dir_all(&nested_dir).unwrap();

    let file1 = temp_dir.path().join("root.ts");
    let file2 = src_dir.join("app.ts");
    let file3 = nested_dir.join("deep.ts");
    let non_ts = src_dir.join("readme.md");

    fs::write(&file1, "// root").unwrap();
    fs::write(&file2, "// app").unwrap();
    fs::write(&file3, "// deep").unwrap();
    fs::write(&non_ts, "# readme").unwrap();

    // @step When I execute the Glob tool with pattern "**/*.ts"
    let tool = GlobTool::new();
    let result = tool
        .execute(json!({
            "pattern": "**/*.ts",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain all TypeScript files recursively
    assert!(result.content.contains("root.ts"));
    assert!(result.content.contains("app.ts"));
    assert!(result.content.contains("deep.ts"));

    // @step And the result should not contain non-TypeScript files
    assert!(!result.content.contains("readme.md"));
}

/// Scenario: Glob with path parameter limits search to directory
#[tokio::test]
async fn test_glob_path_limits_search_to_directory() {
    // @step Given a project with files in src and lib directories
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let lib_dir = temp_dir.path().join("lib");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&lib_dir).unwrap();

    let src_file = src_dir.join("main.ts");
    let lib_file = lib_dir.join("util.ts");

    fs::write(&src_file, "// src main").unwrap();
    fs::write(&lib_file, "// lib util").unwrap();

    // @step When I execute the Glob tool with pattern "*.ts" and path "src"
    let tool = GlobTool::new();
    let result = tool
        .execute(json!({
            "pattern": "*.ts",
            "path": src_dir.to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should only contain files from the src directory
    assert!(result.content.contains("main.ts"));
    assert!(!result.content.contains("util.ts"));
}

/// Scenario: Glob for non-existent pattern returns no matches
#[tokio::test]
async fn test_glob_nonexistent_pattern_returns_no_matches() {
    // @step Given a project directory
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("normal.rs");
    fs::write(&file, "// code").unwrap();

    // @step When I execute the Glob tool with pattern "nonexistent*.xyz"
    let tool = GlobTool::new();
    let result = tool
        .execute(json!({
            "pattern": "nonexistent*.xyz",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should be "No matches found"
    assert!(result.content.contains("No matches found"));
}

/// Scenario: Glob respects gitignore by default
#[tokio::test]
async fn test_glob_respects_gitignore() {
    // @step Given a project with node_modules directory and .gitignore
    let temp_dir = TempDir::new().unwrap();

    // Initialize a git repo (required for .gitignore to be respected)
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()
        .expect("git init failed");

    let node_modules = temp_dir.path().join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();

    // Create gitignore
    let gitignore = temp_dir.path().join(".gitignore");
    fs::write(&gitignore, "node_modules/\n").unwrap();

    // Create files
    let src_file = temp_dir.path().join("app.js");
    let ignored_file = node_modules.join("dep.js");
    fs::write(&src_file, "// app").unwrap();
    fs::write(&ignored_file, "// dep").unwrap();

    // @step When I execute the Glob tool with pattern "**/*.js"
    let tool = GlobTool::new();
    let result = tool
        .execute(json!({
            "pattern": "**/*.js",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should not include files from node_modules
    assert!(result.content.contains("app.js"));
    assert!(!result.content.contains("dep.js"));
}

// ==========================================
// TOOL REGISTRY INTEGRATION TESTS
// ==========================================

/// Scenario: GrepTool and GlobTool are registered in default ToolRegistry
#[test]
fn test_grep_glob_tools_registered_in_default_registry() {
    // @step Given a default ToolRegistry
    let registry = ToolRegistry::default();

    // @step Then the registry should contain the "Grep" tool
    assert!(registry.get("Grep").is_some());

    // @step And the registry should contain the "Glob" tool
    assert!(registry.get("Glob").is_some());

    // @step And the Grep tool should have the correct name
    let grep = registry.get("Grep").unwrap();
    assert_eq!(grep.name(), "Grep");

    // @step And the Glob tool should have the correct name
    let glob = registry.get("Glob").unwrap();
    assert_eq!(glob.name(), "Glob");
}

/// Scenario: Runner can execute Grep and Glob tools
#[tokio::test]
async fn test_runner_can_execute_grep_and_glob_tools() {
    // @step Given a Runner with default tools
    let runner = Runner::new();
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.rs");
    fs::write(&file, "// TODO: test\n").unwrap();

    // @step When I execute the Grep tool through the runner
    let grep_result = runner
        .execute_tool(
            "Grep",
            json!({
                "pattern": "TODO",
                "path": temp_dir.path().to_string_lossy()
            }),
        )
        .await
        .unwrap();

    // @step Then the execution should succeed
    assert!(!grep_result.is_error);
    assert!(grep_result.content.contains("test.rs"));

    // @step When I execute the Glob tool through the runner
    let glob_result = runner
        .execute_tool(
            "Glob",
            json!({
                "pattern": "**/*.rs",
                "path": temp_dir.path().to_string_lossy()
            }),
        )
        .await
        .unwrap();

    // @step Then the execution should succeed
    assert!(!glob_result.is_error);
    assert!(glob_result.content.contains("test.rs"));
}
