//! Feature: spec/features/nanograph-database-lifecycle.feature
//!
//! COV-057: stats on a freshly initialized, empty graph database. Both
//! production schemas (AST code graph and Learnings graph) are checked
//! so every node AND edge type reported by `GraphDatabase::stats` is
//! zero and the call itself does not error.

use codelet_graph::database::GraphDatabase;
use serde_json::Value;

const AST_CODE_SCHEMA: &str = include_str!("../schemas/ast-code.pg");
const LEARNINGS_SCHEMA: &str = include_str!("../schemas/learnings.pg");

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Returns every `(type, count)` pair in a stats section.
fn counts(stats: &Value, section: &str) -> Result<Vec<(String, u64)>, String> {
    let map = stats
        .get(section)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("stats has no `{section}` object: {stats}"))?;
    map.iter()
        .map(|(name, count)| {
            count
                .as_u64()
                .map(|c| (name.clone(), c))
                .ok_or_else(|| format!("{section}.{name} is not a count: {count}"))
        })
        .collect()
}

/// Scenario: Empty graph returns zero stats without error
#[tokio::test]
async fn scenario_empty_graph_returns_zero_stats_without_error() -> TestResult {
    let temp_dir = tempfile::tempdir()?;

    for (name, schema) in [
        ("ast-code", AST_CODE_SCHEMA),
        ("learnings", LEARNINGS_SCHEMA),
    ] {
        // @step Given the graph database has been initialized with no data loaded
        let db_path = temp_dir.path().join(format!("{name}.nano"));
        let db = GraphDatabase::init(&db_path, schema).await?;

        // @step When a stats query is executed against the graph
        let result = db.stats();

        // @step Then all node and edge counts are zero
        let stats = result.as_ref().map_err(String::clone)?;
        let nodes = counts(stats, "nodes")?;
        let edges = counts(stats, "edges")?;
        assert!(!nodes.is_empty(), "{name}: schema should report node types");
        assert!(!edges.is_empty(), "{name}: schema should report edge types");
        for (type_name, count) in nodes.iter().chain(edges.iter()) {
            assert_eq!(*count, 0, "{name}: {type_name} should be empty");
        }

        // @step And no error is returned
        assert!(
            result.is_ok(),
            "{name}: stats on empty graph must not error"
        );
    }
    Ok(())
}
