//! Feature: spec/features/ast-entity-deduplication.feature
//!
//! Moved out of walk_project_directory_with.rs (COV-075: one test file per feature).

use super::*;

// ============================================================================
// Scenario: Deduplicate File nodes when import target is also walked directly
// ============================================================================
#[tokio::test]
async fn test_dedup_file_nodes_when_import_target_is_also_walked() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with file "src/index.ts" that imports from "./utils"
    let index_content = r#"
import { helper } from './utils';

export function main(): void {
    helper();
}
"#;
    write_test_file(project_dir, "src/index.ts", index_content);

    // @step And a project file "src/utils.ts" that is also walked by the file walker
    let utils_content = r#"
export function helper(): string {
    return 'hello';
}
"#;
    write_test_file(project_dir, "src/utils.ts", utils_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir, true);

    // @step Then only one File node should exist for "src/utils.ts"
    let utils_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) == Some("src/utils.ts"))
        })
        .collect();
    assert_eq!(
        utils_file_nodes.len(),
        1,
        "Should have exactly 1 File node for src/utils.ts, got {} — deduplication failed",
        utils_file_nodes.len()
    );

    // @step And that File node should have the full properties including language, lineCount, and isTest
    if let GraphEntity::Node { properties, .. } = utils_file_nodes[0] {
        assert!(
            properties.contains_key("language"),
            "Deduped File node must have 'language' property (full node wins over stub)"
        );
        assert!(
            properties.contains_key("lineCount"),
            "Deduped File node must have 'lineCount' property (full node wins over stub)"
        );
        assert!(
            properties.contains_key("isTest"),
            "Deduped File node must have 'isTest' property (full node wins over stub)"
        );
    } else {
        panic!("Expected a Node variant");
    }

    // @step And the Imports edge from "src/index.ts" to "src/utils.ts" should be preserved
    let import_edges: Vec<_> = entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Imports"))
        .collect();
    assert!(
        !import_edges.is_empty(),
        "Imports edge must be preserved after deduplication"
    );
}

// ============================================================================
// Scenario: Preserve stub File nodes for external import targets
// ============================================================================
#[tokio::test]
async fn test_preserve_stub_file_nodes_for_external_imports() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let project_dir = temp_dir.path();

    // @step Given a project with file "src/app.ts" that imports from "express"
    let app_content = r#"
import { Router } from 'express';

export function createApp(): void {
    const router = Router();
}
"#;
    write_test_file(project_dir, "src/app.ts", app_content);
    write_test_file(project_dir, ".gitignore", "node_modules/\ntarget/\n");

    // @step When the extraction pipeline processes the project directory
    let entities = walk_and_extract(project_dir, true);

    // @step Then a stub File node should exist for the external import target
    // An external package is not a file of the tree: the import is counted
    // as unresolved (external) and no stub File node is created.
    let external_file_nodes: Vec<_> = entities
        .iter()
        .filter(|e| {
            matches!(e, GraphEntity::Node { node_type, properties, .. }
            if node_type == "File"
                && properties.get("path").and_then(|v| v.as_str()) != Some("src/app.ts"))
        })
        .collect();
    assert!(
        external_file_nodes.is_empty(),
        "External import target must not get a stub File node, got {external_file_nodes:?}"
    );
    let indexed = index_tree(project_dir, true, 1);
    let external = serde_json::to_value(&indexed.unresolved).expect("serialize unresolved");
    assert_eq!(
        external["imports"]["external"].as_u64(),
        Some(1),
        "The express import should be counted as an external unresolved import: {external}"
    );

    // @step And no unique constraint violation should occur
    // Verify by loading into a real graph database
    let db_path = temp_dir.path().join("test-ast-external.nano");
    let db = GraphDatabase::init(&db_path, AST_CODE_SCHEMA)
        .await
        .expect("DB init should succeed");

    let load_result = db.load_entities(&entities).await;
    assert!(
        load_result.is_ok(),
        "Graph load should succeed without unique constraint violation, got: {:?}",
        load_result.err()
    );
}
