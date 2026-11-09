// Feature: spec/features/ast-extraction-pipeline.feature
use super::*;

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
    let entities = walk_and_extract(project_dir, false);

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
    let entities = walk_and_extract(project_dir, false);

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
    let entities = walk_and_extract(project_dir, true);

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
