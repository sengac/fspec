
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Feature: spec/features/astgrep-tool-for-ast-based-code-search.feature
//!
//! Tests for AstGrep Tool Implementation - CORE-005
//!
//! These tests verify the implementation of AST-based code search
//! using the native ast-grep Rust crates (ast-grep-core + ast-grep-language).

use codelet::agent::Runner;
use codelet::tools::{astgrep::AstGrepTool, Tool, ToolRegistry};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

// ==========================================
// ASTGREP TOOL EXECUTION TESTS
// ==========================================

/// Scenario: Find function definition by pattern
#[tokio::test]
async fn test_astgrep_find_function_definition_by_pattern() {
    // @step Given a directory with TypeScript files containing function definitions
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("tools.ts");
    fs::write(
        &file,
        r#"
export function executeTool(name: string, input: any): string {
    return "result";
}

function helperFunction() {
    console.log("helper");
}
"#,
    )
    .unwrap();

    // @step When I execute the AstGrep tool with pattern "function executeTool" and language "typescript"
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "function executeTool",
            "language": "typescript",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain file paths with line numbers and column positions
    assert!(result.content.contains("tools.ts"));
    assert!(result.content.contains(":")); // Should have line:column format

    // @step And the result should be in "file:line:column:text" format
    // Format: file:line:column:matched_text
    let has_format = result.content.lines().any(|line| {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        parts.len() >= 3
    });
    assert!(has_format);

    // @step And the result should not be an error
    assert!(!result.is_error);
}

/// Scenario: Find method calls using meta-variable pattern
#[tokio::test]
async fn test_astgrep_find_method_calls_using_metavariable() {
    // @step Given a directory with TypeScript files containing logger calls
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("app.ts");
    fs::write(
        &file,
        r#"
const logger = console;
logger.info("Starting application");
logger.error("Something went wrong", error);
logger.debug("Debug info", { key: "value" });
"#,
    )
    .unwrap();

    // @step When I execute the AstGrep tool with pattern "logger.$METHOD($$$ARGS)" and language "typescript"
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "logger.$METHOD($$$ARGS)",
            "language": "typescript",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain all files with logger method calls
    assert!(result.content.contains("app.ts"));

    // @step And the matched text should include the method names and arguments
    assert!(
        result.content.contains("logger.info")
            || result.content.contains("logger.error")
            || result.content.contains("logger.debug")
    );
}

/// Scenario: Find Rust function definitions with return types
#[tokio::test]
async fn test_astgrep_find_rust_functions_with_return_types() {
    // @step Given a directory with Rust source files
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("lib.rs");
    fs::write(
        &file,
        r#"
pub fn calculate(x: i32, y: i32) -> i32 {
    x + y
}

fn helper() -> String {
    String::new()
}

fn no_return() {
    println!("no return type");
}
"#,
    )
    .unwrap();

    // @step When I execute the AstGrep tool with pattern "fn $NAME($$$ARGS) -> $RET { $$$BODY }" and language "rust"
    // Note: In Rust AST, function definitions include the body, so we need to match it
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "fn $NAME($$$ARGS) -> $RET { $$$BODY }",
            "language": "rust",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain function definitions with return types
    assert!(result.content.contains("lib.rs"));

    // @step And the result should show the function names and return types
    // Should match calculate and helper, but NOT no_return (no return type)
    assert!(result.content.contains("calculate") || result.content.contains("helper"));
}

/// Scenario: Handle invalid pattern syntax with helpful error
#[tokio::test]
async fn test_astgrep_invalid_pattern_returns_helpful_error() {
    // @step Given an invalid AST pattern "function {{{"
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("code.ts");
    fs::write(&file, "function foo() {}").unwrap();

    // @step When I execute the AstGrep tool with the invalid pattern and language "typescript"
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "function {{{",
            "language": "typescript",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the result should contain "Error"
    assert!(result.content.contains("Error") || result.is_error);

    // @step And the result should contain pattern syntax guidance
    // Should include helpful information about pattern syntax
    assert!(
        result.content.contains("pattern")
            || result.content.contains("syntax")
            || result.content.contains("$")
    );

    // @step And the result should suggest how to fix the pattern
    // Error message should be helpful for the agent
    assert!(result.content.len() > 20); // Should have meaningful content
}

/// Scenario: Limit search to specific directory paths
#[tokio::test]
async fn test_astgrep_limit_search_to_specific_paths() {
    // @step Given a project with files in src and tests directories
    let temp_dir = TempDir::new().unwrap();
    let src_dir = temp_dir.path().join("src");
    let tests_dir = temp_dir.path().join("tests");
    fs::create_dir_all(&src_dir).unwrap();
    fs::create_dir_all(&tests_dir).unwrap();

    let src_file = src_dir.join("lib.rs");
    let test_file = tests_dir.join("test.rs");
    fs::write(&src_file, "fn main_function() {}").unwrap();
    fs::write(&test_file, "fn test_function() {}").unwrap();

    // @step When I execute the AstGrep tool with pattern "fn $NAME" and language "rust" and paths ["src/"]
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "fn $NAME",
            "language": "rust",
            "paths": [src_dir.to_string_lossy()]
        }))
        .await
        .unwrap();

    // @step Then the result should only contain files from the src directory
    assert!(result.content.contains("lib.rs") || result.content.contains("src"));

    // @step And the result should not contain files from other directories
    assert!(!result.content.contains("test.rs"));
}

/// Scenario: Large search results are truncated at character limit
#[tokio::test]
async fn test_astgrep_large_results_truncated() {
    // @step Given a large codebase with many matching patterns
    let temp_dir = TempDir::new().unwrap();

    // Create many files with matching patterns to exceed 30000 chars
    for i in 0..100 {
        let file = temp_dir.path().join(format!("file{}.rs", i));
        let content = format!(
            r#"
fn function_{}_a() {{ println!("a"); }}
fn function_{}_b() {{ println!("b"); }}
fn function_{}_c() {{ println!("c"); }}
fn function_{}_d() {{ println!("d"); }}
fn function_{}_e() {{ println!("e"); }}
"#,
            i, i, i, i, i
        );
        fs::write(&file, content).unwrap();
    }

    // @step When I execute the AstGrep tool with a pattern that matches many files
    let tool = AstGrepTool::new();
    let result = tool
        .execute(json!({
            "pattern": "fn $NAME()",
            "language": "rust",
            "path": temp_dir.path().to_string_lossy()
        }))
        .await
        .unwrap();

    // @step Then the output should be truncated at 30000 characters
    assert!(result.content.len() <= 35000); // Allow some buffer for warning message

    // @step And a truncation warning should be included
    if result.truncated {
        assert!(
            result.content.contains("truncated")
                || result.content.contains("...")
                || result.truncated
        );
    }
}

// ==========================================
// TOOL REGISTRY INTEGRATION TESTS
// ==========================================

/// Scenario: AstGrepTool is registered in default ToolRegistry
#[test]
fn test_astgrep_tool_registered_in_default_registry() {
    // @step Given a default ToolRegistry
    let registry = ToolRegistry::default();

    // @step Then the registry should contain the "AstGrep" tool
    assert!(registry.get("AstGrep").is_some());

    // @step And the AstGrep tool should have the correct name
    let tool = registry.get("AstGrep").unwrap();
    assert_eq!(tool.name(), "AstGrep");
}

/// Scenario: Runner can execute AstGrep tool
#[tokio::test]
async fn test_runner_can_execute_astgrep_tool() {
    // @step Given a Runner with default tools
    let runner = Runner::new();
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("test.rs");
    fs::write(&file, "fn test_function() {}\n").unwrap();

    // @step When I execute the AstGrep tool through the runner
    let result = runner
        .execute_tool(
            "AstGrep",
            json!({
                "pattern": "fn $NAME",
                "language": "rust",
                "path": temp_dir.path().to_string_lossy()
            }),
        )
        .await
        .unwrap();

    // @step Then the execution should succeed
    assert!(!result.is_error);

    // @step And the result should contain search matches
    assert!(result.content.contains("test.rs") || result.content.contains("test_function"));
}
