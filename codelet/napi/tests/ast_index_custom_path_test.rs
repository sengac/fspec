// Feature: spec/features/ast-extraction-pipeline.feature
//
// AST Index Custom Path — Tests for indexing external directories
// via the `path` parameter on `ast_index`. Verifies that:
// - `walk_and_extract` with `respect_gitignore: false` finds files
//   inside gitignored directories
// - `walk_and_extract` with `respect_gitignore: true` still skips them
// - `AstIndex` deserializes with and without the optional `path` field

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_napi::graph::ast_pipeline::walk_and_extract;
use codelet_napi::graph::graph_entities::GraphEntity;

mod graph_test_helpers;
use graph_test_helpers::{count_nodes, write_test_file};

// ============================================================================
// Scenario: AstIndex deserializes without path (backwards compatible)
// ============================================================================
#[test]
fn test_ast_index_deserializes_without_path() {
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Given a JSON payload for ast_index with no path field
    let json = r#"{"action_type":"ast_index"}"#;

    // @step When the action is deserialized
    let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

    // @step Then it should succeed with path set to None
    assert!(
        result.is_ok(),
        "AstIndex should parse without path: {:?}",
        result.err()
    );
    if let Ok(GraphSearchAction::AstIndex { path, .. }) = result {
        assert!(path.is_none(), "Path should be None when omitted");
    } else {
        panic!("Expected AstIndex variant");
    }
}

// ============================================================================
// Scenario: AstIndex deserializes with explicit path
// ============================================================================
#[test]
fn test_ast_index_deserializes_with_path() {
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Given a JSON payload for ast_index with path "tmp/my-repo"
    let json = r#"{"action_type":"ast_index","path":"tmp/my-repo"}"#;

    // @step When the action is deserialized
    let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

    // @step Then it should succeed with path set to "tmp/my-repo"
    assert!(
        result.is_ok(),
        "AstIndex should parse with path: {:?}",
        result.err()
    );
    if let Ok(GraphSearchAction::AstIndex { path, .. }) = result {
        assert_eq!(path.as_deref(), Some("tmp/my-repo"), "Path should match");
    } else {
        panic!("Expected AstIndex variant");
    }
}

// ============================================================================
// Scenario: AstIndex deserializes with null path
// ============================================================================
#[test]
fn test_ast_index_deserializes_with_null_path() {
    use codelet_tools::graph_search::GraphSearchAction;

    // @step Given a JSON payload for ast_index with path explicitly null
    let json = r#"{"action_type":"ast_index","path":null}"#;

    // @step When the action is deserialized
    let result: Result<GraphSearchAction, _> = serde_json::from_str(json);

    // @step Then it should succeed with path set to None
    assert!(
        result.is_ok(),
        "AstIndex should parse with null path: {:?}",
        result.err()
    );
    if let Ok(GraphSearchAction::AstIndex { path, .. }) = result {
        assert!(path.is_none(), "Null path should deserialize as None");
    } else {
        panic!("Expected AstIndex variant");
    }
}

// ============================================================================
// Scenario: walk_and_extract with respect_gitignore=true skips gitignored dirs
// ============================================================================
#[test]
fn test_walk_and_extract_respects_gitignore_when_enabled() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory with a .gitignore that excludes "external_deps/"
    // Note: We use "external_deps" instead of "vendor" because "vendor" is in SKIP_DIRS
    write_test_file(project_dir, ".gitignore", "external_deps/\n");

    // Initialize a git repo so the ignore crate recognizes .gitignore
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_dir)
        .output()
        .expect("git init");

    // @step And a TypeScript file in the root "src/" directory
    write_test_file(
        project_dir,
        "src/main.ts",
        "export function main(): void { console.log('hello'); }\n",
    );

    // @step And a TypeScript file inside the gitignored "external_deps/" directory
    write_test_file(
        project_dir,
        "external_deps/lib/dep.ts",
        "export function externalDep(): string { return 'dep'; }\n",
    );

    // @step When walk_and_extract runs with respect_gitignore=true
    let entities = walk_and_extract(project_dir, true).expect("walk_and_extract should succeed");

    // @step Then only the "src/main.ts" file should be found
    let file_slugs: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();

    assert!(
        file_slugs.iter().any(|s| s.contains("main")),
        "Should contain src/main.ts, got: {:?}",
        file_slugs
    );

    // @step And the external_deps/lib/dep.ts file should be skipped
    assert!(
        !file_slugs
            .iter()
            .any(|s| s.contains("external") || s.contains("dep")),
        "Should NOT contain files from external_deps/, got: {:?}",
        file_slugs
    );
}

// ============================================================================
// Scenario: walk_and_extract with respect_gitignore=false finds gitignored files
// ============================================================================
#[test]
fn test_walk_and_extract_ignores_gitignore_when_disabled() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory with a .gitignore that excludes "external_deps/"
    // Note: We use "external_deps" instead of "vendor" because "vendor" is in SKIP_DIRS
    write_test_file(project_dir, ".gitignore", "external_deps/\n");

    // Initialize a git repo so the ignore crate would normally apply .gitignore
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_dir)
        .output()
        .expect("git init");

    // @step And a TypeScript file in the root "src/" directory
    write_test_file(
        project_dir,
        "src/main.ts",
        "export function main(): void { console.log('hello'); }\n",
    );

    // @step And a TypeScript file inside the gitignored "external_deps/" directory
    write_test_file(
        project_dir,
        "external_deps/lib/dep.ts",
        "export function externalDep(): string { return 'dep'; }\n",
    );

    // @step When walk_and_extract runs with respect_gitignore=false
    let entities = walk_and_extract(project_dir, false).expect("walk_and_extract should succeed");

    // @step Then both files should be found
    let file_slugs: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();

    assert!(
        file_slugs.iter().any(|s| s.contains("main")),
        "Should contain src/main.ts, got: {:?}",
        file_slugs
    );

    // @step And the external_deps/lib/dep.ts file should also be found
    assert!(
        file_slugs.iter().any(|s| s.contains("dep")),
        "Should contain external_deps/lib/dep.ts when gitignore disabled, got: {:?}",
        file_slugs
    );
}

// ============================================================================
// Scenario: walk_and_extract still skips SKIP_DIRS even with gitignore disabled
// ============================================================================
#[test]
fn test_walk_and_extract_still_skips_hardcoded_dirs_with_gitignore_disabled() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with files in node_modules and target (hardcoded skip dirs)
    write_test_file(
        project_dir,
        "src/app.ts",
        "export function app(): void {}\n",
    );
    write_test_file(
        project_dir,
        "node_modules/pkg/index.ts",
        "export function pkg(): void {}\n",
    );
    write_test_file(project_dir, "target/debug/main.rs", "fn compiled() {}\n");

    // @step When walk_and_extract runs with respect_gitignore=false
    let entities = walk_and_extract(project_dir, false).expect("walk_and_extract should succeed");

    // @step Then src/app.ts should be found
    let file_slugs: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();

    assert!(
        file_slugs.iter().any(|s| s.contains("app")),
        "Should contain src/app.ts, got: {:?}",
        file_slugs
    );

    // @step And node_modules files should still be skipped (hardcoded SKIP_DIRS)
    assert!(
        !file_slugs
            .iter()
            .any(|s| s.contains("node_modules") || s.contains("node-modules")),
        "Should NOT contain files from node_modules even with gitignore disabled, got: {:?}",
        file_slugs
    );

    // @step And target files should still be skipped (hardcoded SKIP_DIRS)
    assert!(
        !file_slugs.iter().any(|s| s.contains("target")),
        "Should NOT contain files from target even with gitignore disabled, got: {:?}",
        file_slugs
    );
}

// ============================================================================
// Scenario: walk_and_extract with respect_gitignore=false extracts functions
//           from files in gitignored directories
// ============================================================================
#[test]
fn test_walk_and_extract_extracts_functions_from_gitignored_dirs() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a .gitignore that excludes "external/"
    write_test_file(project_dir, ".gitignore", "external/\n");

    // Initialize git repo so .gitignore is recognized
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_dir)
        .output()
        .expect("git init");

    // @step And an external Python project in the gitignored "external/" directory
    write_test_file(
        project_dir,
        "external/mylib/core.py",
        r#"
def process_data(items):
    return [x * 2 for x in items]

class DataProcessor:
    def __init__(self, config):
        self.config = config
    
    def run(self):
        pass
"#,
    );

    // @step When walk_and_extract runs with respect_gitignore=false
    let entities = walk_and_extract(project_dir, false).expect("walk_and_extract should succeed");

    // @step Then Function nodes should be extracted from the gitignored Python files
    let function_count = count_nodes(&entities, "Function");
    assert!(
        function_count >= 2,
        "Should extract at least 2 functions (process_data, run, __init__), got {}",
        function_count
    );

    // @step And Type nodes should be extracted for the DataProcessor class
    let type_count = count_nodes(&entities, "Type");
    assert!(
        type_count >= 1,
        "Should extract at least 1 type (DataProcessor), got {}",
        type_count
    );
}

// ============================================================================
// Scenario: walk_and_extract with respect_gitignore=false indexes multiple
//           languages in a gitignored directory
// ============================================================================
#[test]
fn test_walk_and_extract_multi_language_in_gitignored_dir() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a .gitignore that excludes "repos/"
    write_test_file(project_dir, ".gitignore", "repos/\n");

    // Initialize git repo so .gitignore is recognized
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_dir)
        .output()
        .expect("git init");

    // @step And a TypeScript file in repos/ts-project/
    write_test_file(
        project_dir,
        "repos/ts-project/src/index.ts",
        "export function greet(name: string): string { return `Hello ${name}`; }\n",
    );

    // @step And a Rust file in repos/rs-project/
    write_test_file(
        project_dir,
        "repos/rs-project/src/lib.rs",
        "pub fn compute(x: i32) -> i32 { x * 2 }\n",
    );

    // @step And a Go file in repos/go-project/
    write_test_file(
        project_dir,
        "repos/go-project/main.go",
        "package main\n\nfunc main() {\n\tfmt.Println(\"hello\")\n}\n",
    );

    // @step When walk_and_extract runs with respect_gitignore=false
    let entities = walk_and_extract(project_dir, false).expect("walk_and_extract should succeed");

    // @step Then File nodes should include files from all three languages
    let file_slugs: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();

    assert!(
        file_slugs
            .iter()
            .any(|s| s.contains("index") && s.contains("ts")),
        "Should contain TypeScript file, got: {:?}",
        file_slugs
    );
    assert!(
        file_slugs
            .iter()
            .any(|s| s.contains("lib") && s.contains("rs")),
        "Should contain Rust file, got: {:?}",
        file_slugs
    );
    assert!(
        file_slugs
            .iter()
            .any(|s| s.contains("main") && s.contains("go")),
        "Should contain Go file, got: {:?}",
        file_slugs
    );

    // @step And Function nodes should be extracted from all three languages
    let fn_names: Vec<String> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type,
                properties,
                ..
            } if node_type == "Function" => properties
                .get("name")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            _ => None,
        })
        .collect();

    assert!(
        fn_names.iter().any(|n| n == "greet"),
        "Should extract TypeScript function 'greet', got: {:?}",
        fn_names
    );
    assert!(
        fn_names.iter().any(|n| n == "compute"),
        "Should extract Rust function 'compute', got: {:?}",
        fn_names
    );
    assert!(
        fn_names.iter().any(|n| n == "main"),
        "Should extract Go function 'main', got: {:?}",
        fn_names
    );
}

// ============================================================================
// Scenario: walk_and_extract with respect_gitignore=true on same directory
//           extracts nothing from gitignored paths
// ============================================================================
#[test]
fn test_walk_and_extract_gitignore_true_extracts_nothing_from_gitignored() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a .gitignore that excludes "external_repos/"
    // Note: We use "external_repos" instead of "vendor" because "vendor" is in SKIP_DIRS
    write_test_file(project_dir, ".gitignore", "external_repos/\n");

    // Initialize a git repo so the ignore crate recognizes .gitignore
    std::process::Command::new("git")
        .args(["init", "--quiet"])
        .current_dir(project_dir)
        .output()
        .expect("git init");

    // @step And source files only inside the gitignored "external_repos/" directory
    write_test_file(
        project_dir,
        "external_repos/app/main.ts",
        "export function start(): void {}\n",
    );

    // @step When walk_and_extract runs with respect_gitignore=true
    let entities = walk_and_extract(project_dir, true).expect("walk_and_extract should succeed");

    // @step Then no File nodes should be found (all files are gitignored)
    let file_count = count_nodes(&entities, "File");
    assert_eq!(
        file_count, 0,
        "Should find no files when all are gitignored, got {}",
        file_count
    );

    // @step And no Function nodes should be extracted
    let fn_count = count_nodes(&entities, "Function");
    assert_eq!(
        fn_count, 0,
        "Should extract no functions when all files are gitignored, got {}",
        fn_count
    );
}
