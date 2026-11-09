// Feature: spec/features/variable-symbol-tracking.feature
use super::*;

// ============================================================================
// Scenario: ast_stats includes variable count after indexing
// ============================================================================
#[tokio::test]
async fn test_ast_stats_includes_variable_count() {
    use codelet_graph::ast_dispatch::dispatch_ast_stats;

    let temp_dir = tempfile::tempdir().expect("temp dir");
    let db = setup_variable_db(temp_dir.path()).await;

    // @step Given a project with files containing module-level variables
    // (setup loaded 9 Variable nodes across 4 files)

    // @step When the project is indexed with ast_index

    // @step Then ast_stats shows the total variable count alongside function and type counts
    let result_json = dispatch_ast_stats(&db).await;
    let result: serde_json::Value = serde_json::from_str(&result_json).expect("valid JSON");

    let nodes = result.get("nodes").expect("nodes field");
    let variable_count = nodes
        .get("Variable")
        .and_then(|v| v.as_u64())
        .expect("Variable count in nodes");

    assert_eq!(variable_count, 9, "Should report 9 Variable nodes");

    let file_count = nodes
        .get("File")
        .and_then(|v| v.as_u64())
        .expect("File count in nodes");
    assert_eq!(file_count, 4, "Should report 4 File nodes");
}
