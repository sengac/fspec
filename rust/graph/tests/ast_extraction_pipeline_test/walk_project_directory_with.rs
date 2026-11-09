// Feature: spec/features/ast-extraction-pipeline.feature
// Feature: spec/features/ast-entity-deduplication.feature
use super::*;

// ============================================================================
// Scenario: Walk project directory with gitignore and batch load
// ============================================================================
#[tokio::test]
async fn test_walk_project_directory_with_gitignore_and_batch_load() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project directory with TypeScript and Rust source files
    write_test_file(
        project_dir,
        "src/main.ts",
        "export function main(): void { console.log('hello'); }\n",
    );
    write_test_file(
        project_dir,
        "src/lib.rs",
        "pub fn init() -> Result<(), String> { Ok(()) }\n",
    );

    // @step And a .gitignore file that excludes "node_modules" and "target"
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");
    write_test_file(
        project_dir,
        "node_modules/dep/index.ts",
        "export function dep(): void {}\n",
    );
    write_test_file(
        project_dir,
        "target/debug/main.rs",
        "fn compiled_main() {}\n",
    );

    // @step When the extraction pipeline walks the project directory
    let all_entities = walk_and_extract(project_dir, true);

    // @step Then files in node_modules should be skipped
    let file_slugs: Vec<String> = all_entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type, slug, ..
            } if node_type == "File" => Some(slug.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !file_slugs
            .iter()
            .any(|s| s.contains("node_modules") || s.contains("node-modules")),
        "Should not contain files from node_modules, got: {:?}",
        file_slugs
    );

    // @step And files in target should be skipped
    assert!(
        !file_slugs.iter().any(|s| s.contains("target")),
        "Should not contain files from target, got: {:?}",
        file_slugs
    );

    // @step And all extracted entities should be loaded in a single batch operation
    // Verify batch by loading into a graph and checking stats
    let db_path = temp_dir.path().join("test-ast.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let jsonl = all_entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node { node_type, properties, .. } => {
                serde_json::to_string(&serde_json::json!({"type": node_type, "data": properties})).ok()
            }
            GraphEntity::Edge { edge_type, from_slug, to_slug, properties } => {
                serde_json::to_string(&serde_json::json!({"edge": edge_type, "from": from_slug, "to": to_slug, "data": properties})).ok()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    db.load_jsonl(&jsonl)
        .await
        .expect("Batch load should succeed");

    // @step And the AST graph should contain File, Function, and Type nodes from the processed files
    let db = db.with_query_source(AST_QUERIES);
    let files = db
        .query("all_files", None)
        .await
        .expect("all_files query should succeed");
    let files_arr = files.as_array().expect("Should be array");
    assert!(
        files_arr.len() >= 2,
        "Should have at least 2 File nodes (main.ts and lib.rs), got {}",
        files_arr.len()
    );

    let functions = db
        .query("all_functions", None)
        .await
        .expect("all_functions query should succeed");
    let fns_arr = functions.as_array().expect("Should be array");
    assert!(
        !fns_arr.is_empty(),
        "Should have extracted at least 1 Function node"
    );
}
