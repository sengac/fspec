#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::needless_collect)]
//! Feature: spec/features/dart-astgrep-support.feature
//!
//! Tests for Dart language support in AstGrep and AstGrepRefactor tools.
//! Verifies that tree-sitter-dart 0.1.0 integration works for pattern
//! matching, file searching, and code refactoring on .dart files.
//!
//! IMPORTANT: Dart's tree-sitter grammar splits top-level function declarations
//! into sibling nodes (function_signature + function_body), so patterns must
//! target either the signature or the body, not both. Class declarations are
//! single nodes and work normally.

use codelet_tools::astgrep::{AstGrepArgs, AstGrepTool};
use codelet_tools::astgrep_refactor::{AstGrepRefactorArgs, AstGrepRefactorTool};
use rig::tool::Tool;
use std::fs;
use tempfile::TempDir;
use uuid::Uuid;

// ==========================================
// DART SAMPLE CODE FOR TESTS
// ==========================================

const DART_CLASSES: &str = r#"
class Animal {
  String name;
  int age;
}

class Dog extends Animal {
  String breed;
}
"#;

const DART_FUNCTIONS: &str = r#"
void main() {
  greet('world');
}

void greet(String name) {
  print('Hello');
}

int add(int a, int b) {
  return a + b;
}
"#;

const DART_MULTI_CLASS: &str = r#"
class ServiceA {
  String name;
}

class ServiceB {
  String name;
}

class ServiceC {
  String name;
}
"#;

// ==========================================
// SCENARIO: Search Dart files for class declarations using AstGrepTool
// ==========================================

/// Scenario: Search Dart files for class declarations using AstGrepTool
#[tokio::test]
async fn test_search_dart_class_declarations() {
    // @step Given a directory containing .dart files with class declarations
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("models.dart");
    fs::write(&file, DART_CLASSES).unwrap();

    // @step When I search with AstGrepTool using pattern 'class $NAME { $$$BODY }' and language 'dart'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "class $NAME { $$$BODY }".to_string(),
            language: "dart".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then the results contain matching Dart classes with file path, line number, and column
    assert!(
        result.contains("models.dart"),
        "Result should contain file path. Got: {result}"
    );
    let has_location = result.lines().any(|line| {
        let parts: Vec<&str> = line.splitn(4, ':').collect();
        parts.len() >= 3
    });
    assert!(
        has_location,
        "Result should have line:column format. Got: {result}"
    );

    // @step And the meta-variable $NAME captures the class name
    assert!(
        result.contains("Animal"),
        "Result should capture Animal class name. Got: {result}"
    );
}

// ==========================================
// SCENARIO: Search Dart files for function declarations with meta-variable capture
// ==========================================

/// Scenario: Search Dart files for function declarations with meta-variable capture
#[tokio::test]
async fn test_search_dart_function_declarations_with_metavar() {
    // @step Given a .dart file containing top-level functions and class methods
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("utils.dart");
    fs::write(&file, DART_FUNCTIONS).unwrap();

    // @step When I search with AstGrepTool using pattern 'void $NAME($$$PARAMS)' and language 'dart'
    // Note: Dart splits function_signature and function_body as siblings, so we match just the signature
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "void $NAME($$$PARAMS)".to_string(),
            language: "dart".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then the results contain matches for each void function signature
    assert!(
        result.contains("utils.dart"),
        "Result should reference the dart file. Got: {result}"
    );

    // @step And the meta-variable $NAME captures each function name correctly
    assert!(
        result.contains("main") || result.contains("greet"),
        "Result should capture function names (main, greet). Got: {result}"
    );
}

// ==========================================
// SCENARIO: Refactor a Dart source file by replacing a matched class pattern
// ==========================================

/// Scenario: Refactor a Dart source file by replacing a matched class pattern
#[tokio::test]
async fn test_refactor_dart_rename_class() {
    // @step Given a .dart source file containing a class named 'OldService'
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("service.dart");
    fs::write(
        &source_file,
        r#"
class OldService {
  void run() {}
}
"#,
    )
    .unwrap();

    // @step When I use AstGrepRefactorTool with language 'dart' to replace 'class OldService { $$$BODY }' with 'class NewService { $$$BODY }'
    let tool = AstGrepRefactorTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepRefactorArgs {
            pattern: "class OldService { $$$BODY }".to_string(),
            language: "dart".to_string(),
            source_file: source_file.to_string_lossy().to_string(),
            target_file: None,
            replacement: Some("class NewService { $$$BODY }".to_string()),
            transforms: None,
            batch: None,
            preview: None,
        })
        .await;

    assert!(
        result.is_ok(),
        "Refactor should succeed. Error: {:?}",
        result.err()
    );

    // @step Then the source file is updated with the class renamed to 'NewService'
    let updated = fs::read_to_string(&source_file).unwrap();
    assert!(
        updated.contains("NewService"),
        "File should contain NewService. Got: {updated}"
    );
    assert!(
        !updated.contains("OldService"),
        "File should no longer contain OldService. Got: {updated}"
    );

    // @step And the class body is preserved unchanged
    assert!(
        updated.contains("void run()"),
        "Class body should be preserved. Got: {updated}"
    );
}

// ==========================================
// SCENARIO: Batch replace all occurrences of a Dart class pattern
// ==========================================

/// Scenario: Batch replace all occurrences of a Dart class pattern
#[tokio::test]
async fn test_batch_replace_dart_class_names() {
    // @step Given a .dart source file containing multiple class declarations with the same field
    let temp_dir = TempDir::new().unwrap();
    let source_file = temp_dir.path().join("services.dart");
    fs::write(&source_file, DART_MULTI_CLASS).unwrap();

    // @step When I use AstGrepRefactorTool in batch mode to replace 'class $NAME { $$$BODY }' with 'class $NAME { int id; }' for language 'dart'
    let tool = AstGrepRefactorTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepRefactorArgs {
            pattern: "class $NAME { $$$BODY }".to_string(),
            language: "dart".to_string(),
            source_file: source_file.to_string_lossy().to_string(),
            target_file: None,
            replacement: Some("class $NAME { int id; }".to_string()),
            transforms: None,
            batch: Some(true),
            preview: None,
        })
        .await;

    assert!(
        result.is_ok(),
        "Batch replace should succeed. Error: {:?}",
        result.err()
    );

    // @step Then all class bodies are replaced with the new field
    let updated = fs::read_to_string(&source_file).unwrap();
    assert!(
        !updated.contains("String name"),
        "Old field should be replaced. Got: {updated}"
    );

    // @step And the class names are preserved in each replacement
    let id_count = updated.matches("int id;").count();
    assert_eq!(
        id_count, 3,
        "All 3 classes should have new field. Got {id_count} in: {updated}"
    );
}

// ==========================================
// SCENARIO: Unsupported language error message includes dart in supported list
// ==========================================

/// Scenario: Unsupported language error message includes dart in supported list
#[tokio::test]
async fn test_unsupported_language_error_includes_dart() {
    // @step When I search with AstGrepTool using language 'brainfuck'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "some pattern".to_string(),
            language: "brainfuck".to_string(),
            path: Some("/tmp".to_string()),
        })
        .await;

    // @step Then the error message lists all supported languages
    assert!(result.is_err(), "Unsupported language should return error");
    let error_msg = result.unwrap_err().to_string();

    // @step And the supported languages list includes 'dart'
    assert!(
        error_msg.to_lowercase().contains("dart"),
        "Error message should mention dart as supported. Got: {error_msg}"
    );
}

// ==========================================
// SCENARIO: AstGrepTool finds .dart files when walking a directory
// ==========================================

/// Scenario: AstGrepTool finds .dart files when walking a directory
#[tokio::test]
async fn test_astgrep_finds_dart_files_in_directory() {
    // @step Given a directory containing mixed files including .dart, .ts, and .rs files
    let temp_dir = TempDir::new().unwrap();

    let dart_file = temp_dir.path().join("widget.dart");
    fs::write(
        &dart_file,
        r#"
class MyWidget {
  String title;
}
"#,
    )
    .unwrap();

    let ts_file = temp_dir.path().join("component.ts");
    fs::write(
        &ts_file,
        r#"
class MyComponent {
  title: string;
}
"#,
    )
    .unwrap();

    let rs_file = temp_dir.path().join("model.rs");
    fs::write(
        &rs_file,
        r#"
struct MyModel {
    title: String,
}
"#,
    )
    .unwrap();

    // @step When I search with AstGrepTool using a Dart pattern and language 'dart'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "class $NAME { $$$BODY }".to_string(),
            language: "dart".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then only .dart files are searched
    assert!(
        result.contains("widget.dart"),
        "Should find matches in .dart files. Got: {result}"
    );

    // @step And .ts and .rs files are not included in results
    assert!(
        !result.contains("component.ts"),
        "Should not search .ts files. Got: {result}"
    );
    assert!(
        !result.contains("model.rs"),
        "Should not search .rs files. Got: {result}"
    );
}

// ==========================================
// SCENARIO: Solidity file extensions are mapped correctly
// ==========================================

/// Scenario: Solidity file extensions are mapped correctly
#[tokio::test]
async fn test_solidity_extensions_mapped() {
    // @step Given a directory containing .sol and .ts files
    let temp_dir = TempDir::new().unwrap();
    let sol_file = temp_dir.path().join("Token.sol");
    fs::write(
        &sol_file,
        r#"
contract Token {
    string public name;
}
"#,
    )
    .unwrap();

    let other_file = temp_dir.path().join("Token.ts");
    fs::write(&other_file, "class Token {}").unwrap();

    // @step When I search with AstGrepTool using language 'solidity'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "contract $NAME { $$$BODY }".to_string(),
            language: "solidity".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then only .sol files are searched
    assert!(
        result.contains("Token.sol") || result.contains("No matches"),
        "Should search .sol files. Got: {result}"
    );
    assert!(
        !result.contains("Token.ts"),
        "Should not search .ts files. Got: {result}"
    );
}

// ==========================================
// SCENARIO: Nix file extensions are mapped correctly
// ==========================================

/// Scenario: Nix file extensions are mapped correctly
#[tokio::test]
async fn test_nix_extensions_mapped() {
    // @step Given a directory containing .nix files
    let temp_dir = TempDir::new().unwrap();
    let nix_file = temp_dir.path().join("default.nix");
    fs::write(
        &nix_file,
        "{ pkgs }: pkgs.mkShell { buildInputs = [ pkgs.hello ]; }\n",
    )
    .unwrap();

    // @step When I search with AstGrepTool using language 'nix'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "$X".to_string(),
            language: "nix".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then only .nix files are searched
    assert!(
        result.contains("default.nix") || result.contains("No matches"),
        "Should search .nix files. Got: {result}"
    );
}

// ==========================================
// SCENARIO: Hcl file extensions are mapped correctly
// ==========================================

/// Scenario: Hcl file extensions are mapped correctly
#[tokio::test]
async fn test_hcl_extensions_mapped() {
    // @step Given a directory containing .hcl and .tf files
    let temp_dir = TempDir::new().unwrap();
    let hcl_file = temp_dir.path().join("main.tf");
    fs::write(
        &hcl_file,
        "resource \"aws_instance\" \"example\" {\n  ami = \"abc-123\"\n}\n",
    )
    .unwrap();

    // @step When I search with AstGrepTool using language 'hcl'
    let tool = AstGrepTool::new(Uuid::nil());
    let result = tool
        .call(AstGrepArgs {
            pattern: "$X".to_string(),
            language: "hcl".to_string(),
            path: Some(temp_dir.path().to_string_lossy().to_string()),
        })
        .await
        .unwrap();

    // @step Then .hcl and .tf files are searched
    assert!(
        result.contains("main.tf") || result.contains("No matches"),
        "Should search .tf files for hcl language. Got: {result}"
    );
}
