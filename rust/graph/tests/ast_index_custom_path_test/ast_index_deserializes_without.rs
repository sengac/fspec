// Feature: spec/features/ast-extraction-pipeline.feature
use super::*;

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
    let entities = walk_and_extract(project_dir, true);

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
    let entities = walk_and_extract(project_dir, false);

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
    let entities = walk_and_extract(project_dir, false);

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
