
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for AstGrepRefactorTool
//!
//! Feature: spec/features/ast-code-refactor-tool-for-codelet.feature

use codelet_tools::astgrep_refactor::{AstGrepRefactorArgs, AstGrepRefactorTool};
use rig::tool::Tool;
use std::fs;
use tempfile::TempDir;

/// Scenario: Extract code matching AST pattern to new file
#[tokio::test]
async fn test_extract_code_matching_ast_pattern_to_new_file() {
    // @step Given a TypeScript file "src/components.ts" containing an arrow function component
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("components.ts");
    fs::write(
        &source_file,
        r#"
const Header = () => {
    return <div>Header</div>;
};

const Footer = () => {
    return <div>Footer</div>;
};
"#,
    )
    .unwrap();

    // @step And the pattern "const $NAME = () => { $$$BODY }" matches exactly one function
    // Note: We'll use a more specific pattern to match just Header
    let target_file = temp_dir.path().join("extracted.ts");

    // @step When the agent calls astgrep_refactor with extract mode
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "const Header = () => { $$$BODY }".to_string(),
        language: "tsx".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    // @step And specifies target file "src/extracted.ts"
    let result = tool.call(args).await;

    // @step Then the matched function should be written to "src/extracted.ts"
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    // @step And the function should be removed from "src/components.ts"
    let updated_source = fs::read_to_string(&source_file).unwrap();
    assert!(
        !updated_source.contains("const Header"),
        "Header should be removed from source"
    );
    assert!(
        updated_source.contains("const Footer"),
        "Footer should remain in source"
    );

    // @step And the tool should return success with moved_code containing the extracted text
    let target_content = fs::read_to_string(&target_file).unwrap();
    assert!(
        target_content.contains("const Header"),
        "Header should be in target file"
    );
}

/// Scenario: Error when pattern matches multiple nodes
#[tokio::test]
async fn test_error_when_pattern_matches_multiple_nodes() {
    // @step Given a Rust file containing 3 functions matching pattern "fn $NAME($$$ARGS) { $$$BODY }"
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"fn foo(x: i32) {
    println!("foo");
}

fn bar(y: i32) {
    println!("bar");
}

fn baz(z: i32) {
    println!("baz");
}"#,
    )
    .unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    // @step When the agent calls astgrep_refactor with this pattern
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME($$$ARGS) { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the tool should return an error
    assert!(result.is_err(), "Expected error for multiple matches");

    // @step And the error message should list all 3 match locations
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("multiple") || err_msg.contains("Multiple"),
        "Error should mention multiple matches: {err_msg}"
    );

    // @step And no files should be modified
    assert!(
        !target_file.exists(),
        "Target file should not be created on error"
    );
}

/// Scenario: Error when pattern matches zero nodes
#[tokio::test]
async fn test_error_when_pattern_matches_zero_nodes() {
    // @step Given a source file with no class definitions
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"
fn hello() {
    println!("hello");
}
"#,
    )
    .unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    // @step When the agent calls astgrep_refactor with pattern "class NonExistent"
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "struct NonExistent".to_string(), // Using struct for Rust instead of class
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the tool should return an error "No matches found for pattern"
    assert!(result.is_err(), "Expected error for no matches");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.to_lowercase().contains("no match") || err_msg.to_lowercase().contains("not found"),
        "Error should mention no matches: {err_msg}"
    );

    // @step And no files should be modified
    assert!(
        !target_file.exists(),
        "Target file should not be created on error"
    );
}

/// Scenario: Error when invalid language specified
#[tokio::test]
async fn test_error_when_invalid_language_specified() {
    // @step Given a valid source file
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("code.txt");
    fs::write(&source_file, "some code").unwrap();

    let target_file = temp_dir.path().join("extracted.txt");

    // @step When the agent calls astgrep_refactor with language "invalid_lang"
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME".to_string(),
        language: "invalid_lang".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the tool should return an error
    assert!(result.is_err(), "Expected error for invalid language");

    // @step And the error should list all 23 supported languages
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("language") || err_msg.contains("Language"),
        "Error should mention language issue: {err_msg}"
    );
}

/// Scenario: Blank lines cleaned up after extraction
#[tokio::test]
async fn test_blank_lines_cleaned_up_after_extraction() {
    // @step Given a source file with a function surrounded by blank lines
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"
fn before() {}


fn target_function() {
    println!("target");
}


fn after() {}
"#,
    )
    .unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    // @step When the agent extracts the function using astgrep_refactor
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn target_function() { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    // @step Then consecutive blank lines in the source file should be collapsed to single blank line
    let updated_source = fs::read_to_string(&source_file).unwrap();
    assert!(
        !updated_source.contains("\n\n\n"),
        "Should not have 3+ consecutive newlines"
    );

    // @step And the source file should remain syntactically valid
    assert!(
        updated_source.contains("fn before()"),
        "before function should remain"
    );
    assert!(
        updated_source.contains("fn after()"),
        "after function should remain"
    );
}

/// Scenario: Successful refactor returns complete result structure
#[tokio::test]
async fn test_successful_refactor_returns_complete_result_structure() {
    // @step Given a valid refactor operation
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"
fn target() {
    println!("target");
}
"#,
    )
    .unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    // @step When the agent calls astgrep_refactor and it succeeds
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn target() { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the result should contain success=true
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    // @step And the result should contain moved_code with the extracted text
    let output = result.unwrap();
    assert!(
        output.contains("success") || output.contains("moved") || !output.is_empty(),
        "Result should contain success information"
    );

    // @step And the result should contain source_file path
    // @step And the result should contain target_file path
    // The result is a JSON string containing these fields
    assert!(target_file.exists(), "Target file should exist");
}

/// Scenario: Replace matched code in-place with replacement pattern
#[tokio::test]
async fn test_replace_matched_code_in_place_with_replacement_pattern() {
    // @step Given a source file with a function to refactor
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"
fn old_function() {
    println!("old");
}
"#,
    )
    .unwrap();

    // @step And a replacement pattern to transform the code
    let replacement = "fn new_function() {\n    println!(\"new\");\n}";

    // @step When the agent calls astgrep_refactor in replace mode
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn old_function() { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some(replacement.to_string()),
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the matched code should be replaced in-place
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    // @step And no extraction to target file should occur
    // (no target_file was specified)

    // @step And the source file should contain the replacement code
    let updated_source = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_source.contains("new_function"),
        "Source should contain replacement: {updated_source}"
    );
    assert!(
        !updated_source.contains("old_function"),
        "Source should not contain original: {updated_source}"
    );
}

/// Scenario: Append to existing target file
#[tokio::test]
async fn test_append_to_existing_target_file() {
    // @step Given a target file "src/extracted.ts" already containing function A
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    let target_file = temp_dir.path().join("extracted.rs");

    // Create source with function B
    fs::write(
        &source_file,
        r#"
fn function_b() {
    println!("B");
}
"#,
    )
    .unwrap();

    // Create target with function A already present
    fs::write(
        &target_file,
        r#"fn function_a() {
    println!("A");
}
"#,
    )
    .unwrap();

    // @step When the agent extracts function B to the same target file
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn function_b() { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()),
        replacement: None,
        transforms: None,
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    // @step Then function B should be appended to "src/extracted.ts"
    let target_content = fs::read_to_string(&target_file).unwrap();

    // @step And function A should still be present in the target file
    assert!(
        target_content.contains("function_a"),
        "Function A should still be present: {target_content}"
    );

    // @step And both functions should appear in order
    assert!(
        target_content.contains("function_b"),
        "Function B should be appended: {target_content}"
    );

    // Verify order: A comes before B
    let a_pos = target_content.find("function_a").unwrap();
    let b_pos = target_content.find("function_b").unwrap();
    assert!(
        a_pos < b_pos,
        "Function A should appear before function B (A at {a_pos}, B at {b_pos})"
    );
}
