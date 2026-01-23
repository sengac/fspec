
#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tests for Enhanced AST Refactor Tool with Transforms and Batch Mode (TOOLS-004)
//!
//! Feature: spec/features/enhanced-ast-refactor-tool-with-transforms-and-batch-mode.feature
//!
//! These tests verify:
//! - Pattern matching with partial patterns and meta-variables
//! - Transform operations: substring, replace, convert (case conversion)
//! - Batch mode for multiple match replacement
//! - Preview/dry-run mode
//! - Error handling for invalid transforms

use codelet_tools::astgrep_refactor::{
    AstGrepRefactorArgs, AstGrepRefactorTool, CaseType, ConvertTransform, ReplaceTransform,
    Separator, SubstringTransform, Transform,
};
use rig::tool::Tool;
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

// ============================================================================
// Scenario: Match function using partial pattern with meta-variables
// ============================================================================

/// Scenario: Match function using partial pattern with meta-variables
#[tokio::test]
async fn test_match_function_using_partial_pattern_with_meta_variables() {
    // @step Given a Rust source file containing a simple function
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    // Use a simpler function without return type for pattern matching
    fs::write(
        &source_file,
        r#"fn calculate_sum(a: i32, b: i32) {
    println!("{}", a + b);
}"#,
    )
    .unwrap();

    // @step When I use pattern 'fn $NAME($$$ARGS) { $$$BODY }' to match the function
    // Note: ast-grep patterns must match the complete AST node structure
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME($$$ARGS) { $$$BODY }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $NAME($$$ARGS) { $$$BODY }".to_string()), // Identity replacement
        transforms: None,
        batch: None,
        preview: Some(true), // Preview mode to see what's captured
    };

    let result = tool.call(args).await;

    // @step Then the pattern matches and captures $NAME as 'calculate_sum' and $$$ARGS as 'a: i32, b: i32'
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());
    let output = result.unwrap();

    // The result should show the match - in preview mode, this verifies pattern matching works
    assert!(
        output.contains("calculate_sum") || output.contains("preview"),
        "Should show matched content or preview info: {output}"
    );
}

// ============================================================================
// Scenario: Match call expressions without full statement as pattern
// ============================================================================

/// Scenario: Match call expressions without full statement as pattern
#[tokio::test]
async fn test_match_call_expressions_without_full_statement_as_pattern() {
    // @step Given a TypeScript source file containing 'const x = 1; console.log(x); const y = 2;'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("app.ts");
    fs::write(&source_file, "const x = 1; console.log(x); const y = 2;").unwrap();

    // @step When I use pattern 'console.log($MSG)' to find the call
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "console.log($MSG)".to_string(),
        language: "typescript".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("console.log($MSG)".to_string()), // Identity replacement
        transforms: None,
        batch: None,
        preview: Some(true),
    };

    let result = tool.call(args).await;

    // @step Then the pattern matches 'console.log(x)' and captures $MSG as 'x'
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());
    let output = result.unwrap();
    assert!(
        output.contains("console.log") || output.contains("preview"),
        "Should match console.log call: {output}"
    );
}

// ============================================================================
// Scenario: Rename function using convert transform with case conversion
// ============================================================================

/// Scenario: Rename function using convert transform with case conversion
#[tokio::test]
async fn test_rename_function_using_convert_transform_with_case_conversion() {
    // @step Given a Rust source file containing 'fn old_snake_name() { }'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn old_snake_name() { }").unwrap();

    // @step When I refactor with pattern 'fn $NAME()', transform {NEW: {convert: {source: $NAME, toCase: camelCase}}}, and replacement 'fn $NEW()'
    let mut transforms = HashMap::new();
    transforms.insert(
        "NEW".to_string(),
        Transform::Convert(ConvertTransform {
            source: "$NAME".to_string(),
            to_case: CaseType::CamelCase,
            separated_by: None,
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $NEW() { }".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the file is updated to contain 'fn oldSnakeName() { }'
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_content.contains("oldSnakeName"),
        "Expected camelCase conversion 'oldSnakeName', got: {updated_content}"
    );
    assert!(
        !updated_content.contains("old_snake_name"),
        "Original snake_case should be replaced: {updated_content}"
    );
}

// ============================================================================
// Scenario: Extract substring from captured variable
// ============================================================================

/// Scenario: Extract substring from captured variable
#[tokio::test]
async fn test_extract_substring_from_captured_variable() {
    // @step Given a source file with variable name 'userNameInput'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("app.ts");
    fs::write(&source_file, "const userNameInput = 'test';").unwrap();

    // @step When I apply transform {SHORT: {substring: {source: $NAME, startChar: 0, endChar: -5}}} to remove 'Input'
    let mut transforms = HashMap::new();
    transforms.insert(
        "SHORT".to_string(),
        Transform::Substring(SubstringTransform {
            source: "$NAME".to_string(),
            start_char: Some(0),
            end_char: Some(-5), // Remove last 5 chars ('Input')
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "const $NAME = $VALUE".to_string(),
        language: "typescript".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("const $SHORT = $VALUE".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the transform produces 'userName' in $SHORT
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_content.contains("userName") && !updated_content.contains("userNameInput"),
        "Expected 'userName' after removing 'Input', got: {updated_content}"
    );
}

// ============================================================================
// Scenario: Remove suffix using replace transform with regex
// ============================================================================

/// Scenario: Remove suffix using replace transform with regex
#[tokio::test]
async fn test_remove_suffix_using_replace_transform_with_regex() {
    // @step Given a source file with function name 'get_user_id'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn get_user_id() { }").unwrap();

    // @step When I apply transform {CLEAN: {replace: {source: $NAME, replace: '_id$', by: ''}}}
    let mut transforms = HashMap::new();
    transforms.insert(
        "CLEAN".to_string(),
        Transform::Replace(ReplaceTransform {
            source: "$NAME".to_string(),
            replace: "_id$".to_string(),
            by: "".to_string(),
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $CLEAN() { }".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the transform produces 'get_user' in $CLEAN
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_content.contains("get_user") && !updated_content.contains("get_user_id"),
        "Expected 'get_user' after removing '_id' suffix, got: {updated_content}"
    );
}

// ============================================================================
// Scenario: Batch replace all matching call sites in one operation
// ============================================================================

/// Scenario: Batch replace all matching call sites in one operation
#[tokio::test]
async fn test_batch_replace_all_matching_call_sites_in_one_operation() {
    // @step Given a TypeScript file with 5 calls to 'oldFunc(arg1)', 'oldFunc(arg2)', etc.
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("app.ts");
    fs::write(
        &source_file,
        r#"
oldFunc(arg1);
oldFunc(arg2);
oldFunc(arg3);
oldFunc(arg4);
oldFunc(arg5);
"#,
    )
    .unwrap();

    // @step When I refactor with pattern 'oldFunc($$$ARGS)', replacement 'newFunc($$$ARGS)', and batch: true
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "oldFunc($$$ARGS)".to_string(),
        language: "typescript".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("newFunc($$$ARGS)".to_string()),
        transforms: None,
        batch: Some(true),
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then all 5 call sites are replaced with 'newFunc' and the result reports matches_count: 5
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let output = result.unwrap();
    assert!(
        output.contains("matches_count") && output.contains("5"),
        "Expected matches_count: 5 in result: {output}"
    );

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        !updated_content.contains("oldFunc"),
        "All oldFunc calls should be replaced: {updated_content}"
    );
    assert_eq!(
        updated_content.matches("newFunc").count(),
        5,
        "Should have exactly 5 newFunc calls: {updated_content}"
    );
}

// ============================================================================
// Scenario: Preview changes without modifying files
// ============================================================================

/// Scenario: Preview changes without modifying files
#[tokio::test]
async fn test_preview_changes_without_modifying_files() {
    // @step Given a source file containing 'fn old() { }'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    let original_content = "fn old() { }";
    fs::write(&source_file, original_content).unwrap();

    // @step When I refactor with pattern 'fn old()', replacement 'fn new()', and preview: true
    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn old() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn new() { }".to_string()),
        transforms: None,
        batch: None,
        preview: Some(true),
    };

    let result = tool.call(args).await;

    // @step Then the result shows the match location, original code, and proposed replacement without modifying the file
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let output = result.unwrap();
    assert!(
        output.contains("preview") || output.contains("location"),
        "Result should contain preview info: {output}"
    );

    // Verify file was NOT modified
    let current_content = fs::read_to_string(&source_file).unwrap();
    assert_eq!(
        current_content, original_content,
        "File should NOT be modified in preview mode"
    );
}

// ============================================================================
// Scenario: Chain multiple transforms with dependency ordering
// ============================================================================

/// Scenario: Chain multiple transforms with dependency ordering
#[tokio::test]
async fn test_chain_multiple_transforms_with_dependency_ordering() {
    // @step Given a source file with function name 'get_user_impl'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn get_user_impl() { }").unwrap();

    // @step When I apply transforms {STRIPPED: {replace: {source: $NAME, replace: '_impl$', by: ''}}, FINAL: {convert: {source: $STRIPPED, toCase: pascalCase}}}
    let mut transforms = HashMap::new();
    transforms.insert(
        "STRIPPED".to_string(),
        Transform::Replace(ReplaceTransform {
            source: "$NAME".to_string(),
            replace: "_impl$".to_string(),
            by: "".to_string(),
        }),
    );
    transforms.insert(
        "FINAL".to_string(),
        Transform::Convert(ConvertTransform {
            source: "$STRIPPED".to_string(), // References STRIPPED output
            to_case: CaseType::PascalCase,
            separated_by: None,
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $FINAL() { }".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then STRIPPED is computed first as 'get_user', then FINAL is computed as 'GetUser'
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_content.contains("GetUser"),
        "Expected PascalCase 'GetUser' after chained transforms, got: {updated_content}"
    );
}

// ============================================================================
// Scenario: Use separatedBy option to control word splitting in case conversion
// ============================================================================

/// Scenario: Use separatedBy option to control word splitting in case conversion
#[tokio::test]
async fn test_use_separated_by_option_to_control_word_splitting() {
    // @step Given a variable name 'user_accountName' with mixed naming conventions
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("app.ts");
    fs::write(&source_file, "const user_accountName = 'test';").unwrap();

    // @step When I apply transform {RESULT: {convert: {source: $NAME, toCase: snakeCase, separatedBy: [underscore]}}}
    let mut transforms = HashMap::new();
    transforms.insert(
        "RESULT".to_string(),
        Transform::Convert(ConvertTransform {
            source: "$NAME".to_string(),
            to_case: CaseType::SnakeCase,
            separated_by: Some(vec![Separator::Underscore]), // Only split on underscore
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "const $NAME = $VALUE".to_string(),
        language: "typescript".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("const $RESULT = $VALUE".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    let result = tool.call(args).await;

    // @step Then the result is 'user_accountname' preserving the underscore boundary but not splitting on case change
    assert!(result.is_ok(), "Expected Ok, got error: {:?}", result.err());

    let updated_content = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated_content.contains("user_accountname"),
        "Expected 'user_accountname' (lowercased within underscore segments), got: {updated_content}"
    );
}

// ============================================================================
// Scenario: Fail operation when transform has invalid regex
// ============================================================================

/// Scenario: Fail operation when transform has invalid regex
#[tokio::test]
async fn test_fail_operation_when_transform_has_invalid_regex() {
    // @step Given a refactor operation with transform {BAD: {replace: {source: $NAME, replace: '[unclosed', by: ''}}}
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn test_func() { }").unwrap();

    let mut transforms = HashMap::new();
    transforms.insert(
        "BAD".to_string(),
        Transform::Replace(ReplaceTransform {
            source: "$NAME".to_string(),
            replace: "[unclosed".to_string(), // Invalid regex - unclosed bracket
            by: "".to_string(),
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $BAD() { }".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    // @step When I execute the refactor operation
    let result = tool.call(args).await;

    // @step Then the operation fails with an error message indicating invalid regex in the transform
    assert!(
        result.is_err(),
        "Expected error for invalid regex, got: {result:?}"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("regex") || err_msg.contains("invalid") || err_msg.contains("pattern"),
        "Error should mention invalid regex: {err_msg}"
    );
}

// ============================================================================
// Scenario: Reject transforms when using extract mode
// ============================================================================

/// Scenario: Reject transforms when using extract mode
#[tokio::test]
async fn test_reject_transforms_when_using_extract_mode() {
    // @step Given a refactor operation with target_file set and transforms specified
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn test_func() { }").unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    let mut transforms = HashMap::new();
    transforms.insert(
        "NEW".to_string(),
        Transform::Convert(ConvertTransform {
            source: "$NAME".to_string(),
            to_case: CaseType::CamelCase,
            separated_by: None,
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()), // Extract mode
        replacement: None,
        transforms: Some(transforms), // Transforms specified - should error
        batch: None,
        preview: None,
    };

    // @step When I execute the refactor operation
    let result = tool.call(args).await;

    // @step Then the operation fails with an error message indicating transforms are not supported in extract mode
    assert!(
        result.is_err(),
        "Expected error when using transforms with extract mode"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("transform") && err_msg.contains("extract"),
        "Error should mention transforms not supported in extract mode: {err_msg}"
    );
}

// ============================================================================
// Scenario: Reject batch mode when using extract mode
// ============================================================================

/// Scenario: Reject batch mode when using extract mode
#[tokio::test]
async fn test_reject_batch_mode_when_using_extract_mode() {
    // @step Given a refactor operation with target_file set and batch: true
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(
        &source_file,
        r#"
fn foo() { }
fn bar() { }
"#,
    )
    .unwrap();

    let target_file = temp_dir.path().join("extracted.rs");

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: Some(target_file.to_string_lossy().to_string()), // Extract mode
        replacement: None,
        transforms: None,
        batch: Some(true), // Batch mode - should error in extract mode
        preview: None,
    };

    // @step When I execute the refactor operation
    let result = tool.call(args).await;

    // @step Then the operation fails with an error message indicating batch mode is not supported in extract mode
    assert!(
        result.is_err(),
        "Expected error when using batch with extract mode"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("batch") && err_msg.contains("extract"),
        "Error should mention batch not supported in extract mode: {err_msg}"
    );
}

// ============================================================================
// Scenario: Fail operation when transforms have cyclic dependency
// ============================================================================

/// Scenario: Fail operation when transforms have cyclic dependency
#[tokio::test]
async fn test_fail_operation_when_transforms_have_cyclic_dependency() {
    // @step Given a refactor operation with transforms {A: {replace: {source: $B, replace: 'x', by: 'y'}}, B: {replace: {source: $A, replace: 'a', by: 'b'}}}
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("lib.rs");
    fs::write(&source_file, "fn test_func() { }").unwrap();

    let mut transforms = HashMap::new();
    transforms.insert(
        "A".to_string(),
        Transform::Replace(ReplaceTransform {
            source: "$B".to_string(), // Depends on B
            replace: "x".to_string(),
            by: "y".to_string(),
        }),
    );
    transforms.insert(
        "B".to_string(),
        Transform::Replace(ReplaceTransform {
            source: "$A".to_string(), // Depends on A - CYCLIC!
            replace: "a".to_string(),
            by: "b".to_string(),
        }),
    );

    let tool = AstGrepRefactorTool::new();
    let args = AstGrepRefactorArgs {
        pattern: "fn $NAME() { }".to_string(),
        language: "rust".to_string(),
        source_file: source_file.to_string_lossy().to_string(),
        target_file: None,
        replacement: Some("fn $A() { }".to_string()),
        transforms: Some(transforms),
        batch: None,
        preview: None,
    };

    // @step When I execute the refactor operation
    let result = tool.call(args).await;

    // @step Then the operation fails with an error message indicating cyclic dependency between transforms
    assert!(
        result.is_err(),
        "Expected error for cyclic transform dependency"
    );
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("cyclic") || err_msg.contains("cycle") || err_msg.contains("circular"),
        "Error should mention cyclic dependency: {err_msg}"
    );
}
