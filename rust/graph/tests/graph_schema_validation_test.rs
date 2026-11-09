//! Feature: spec/features/graph-compaction-schema-migration.feature
//!
//! Moved out of graph_reset_test.rs (COV-075: one test file per feature).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::needless_collect
)]

use codelet_graph::database::GraphDatabase;
use std::path::PathBuf;

/// Minimal valid schema for testing.
const TEST_SCHEMA: &str = r#"
node File {
path: String @key
}
"#;

/// Create a temporary database directory path.
fn temp_db_path(suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "fspec-graph-reset-test-{}-{}",
        suffix,
        std::process::id()
    ));
    // Clean up any leftover from previous runs
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

// ============================================================================
// Additional: Schema match passes validation (no error)
// ============================================================================
#[tokio::test]
async fn test_schema_match_passes_validation() {
    let db_path = temp_db_path("schema-match");

    // @step Given the bundled schema hash matches the on-disk schema.pg hash
    let _db = GraphDatabase::init(&db_path, TEST_SCHEMA)
        .await
        .expect("init should succeed");

    // @step When the database is opened
    let result = GraphDatabase::open_or_init_with_schema_check(&db_path, TEST_SCHEMA).await;
    // @step Then no migration is performed
    // (a matching hash goes straight to open; there is no migration path)
    // @step And the database opens normally
    assert!(
        result.is_ok(),
        "Same schema should open successfully: {:?}",
        result.err()
    );

    // Cleanup
    let _ = std::fs::remove_dir_all(&db_path);
}
