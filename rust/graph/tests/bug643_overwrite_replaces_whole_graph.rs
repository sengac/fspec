//! BUG-643: a nanograph Overwrite load replaces the whole graph.
//!
//! Feature: spec/features/an-overwrite-load-replaces-the-whole-graph.feature
//!
//! In nanograph 1.3.0 an Overwrite load kept the rows of a keyed node type
//! whose keys were absent from the new load (the vendored fix is in
//! rust/patches/nanograph.patch). Every code graph here is built by indexing
//! a temp project with `ast_index`; the learnings graphs are temp databases.

use std::path::Path;

use codelet_graph::ast_dispatch::{dispatch_ast_search, dispatch_ast_stats, AST_QUERIES};
use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::database::GraphDatabase;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::learnings_dispatch::{dispatch_learnings_search, dispatch_learnings_stats};
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError, TestResult};
use serde_json::{json, Value};

pub mod code_graph_fixture;
use code_graph_fixture::{entities, index, project};

fn parsed(output: &str) -> Result<Value, TestError> {
    Ok(serde_json::from_str(output)?)
}

/// The sorted `name`s of the rows of a query result or search result.
fn names(rows: &Value) -> Result<Vec<String>, TestError> {
    let mut found: Vec<String> = rows
        .as_array()
        .ok_or_else(|| format!("not a row list: {rows}"))?
        .iter()
        .filter_map(|row| row["name"].as_str().map(str::to_owned))
        .collect();
    found.sort();
    Ok(found)
}

/// (node type, name or title) of every node, and the count of edges, sorted.
fn node_names(all: &[GraphEntity]) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = all
        .iter()
        .filter_map(|entity| match entity {
            GraphEntity::Node {
                node_type,
                properties,
                ..
            } => {
                let name = properties
                    .get("name")
                    .or_else(|| properties.get("path"))
                    .or_else(|| properties.get("title"))?
                    .as_str()?;
                Some((node_type.clone(), name.to_string()))
            }
            GraphEntity::Edge { .. } => None,
        })
        .collect();
    found.sort();
    found
}

const LIB_A: (&str, &str) = ("src/lib.rs", "pub fn a() -> u32 {\n    1\n}\n");
const LIB_B: (&str, &str) = ("src/lib.rs", "pub fn b() -> u32 {\n    1\n}\n");

/// What the code graph of `root` holds: the function query, the export's
/// nodes and the stats' Function count.
async fn code_graph_view(
    root: &Path,
) -> Result<(Vec<String>, Vec<(String, String)>, Value), TestError> {
    let graph = registry::read_code_graph(root).await?;
    let functions = names(
        &graph
            .query_with_source(AST_QUERIES, "all_functions", None)
            .await?,
    )?;
    let exported = node_names(&graph.export_all_entities().await?);
    let stats = parsed(&dispatch_ast_stats(&graph).await)?;
    Ok((functions, exported, stats["nodes"]["Function"].clone()))
}

fn file_and_b() -> Vec<(String, String)> {
    vec![
        ("File".to_string(), "src/lib.rs".to_string()),
        ("Function".to_string(), "b".to_string()),
    ]
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_renamed_definition_is_the_only_one_left_after_the_project_is_indexed_again() -> TestResult
{
    // @step Given a project whose src/lib.rs holds fn a has been indexed
    let (_directory, root) = project(&[LIB_A])?;
    index(&root).await?;

    // @step When a is renamed to b and ast_index runs for the project again without reset
    std::fs::write(root.join(LIB_B.0), LIB_B.1)?;
    index(&root).await?;

    // @step Then ast_search lists b and not a
    let graph = registry::read_code_graph(&root).await?;
    let search = parsed(
        &dispatch_ast_search(
            &graph,
            "",
            Some("Function"),
            Some(50),
            None,
            None,
            None,
            None,
        )
        .await,
    )?;
    assert_eq!(names(&search["results"])?, ["b"], "{search}");

    // @step And the graph's export holds one Function node, b
    let exported = node_names(&graph.export_all_entities().await?);
    assert_eq!(exported, file_and_b());

    // @step And ast_stats counts 1 Function
    let stats = parsed(&dispatch_ast_stats(&graph).await)?;
    assert_eq!(stats["nodes"]["Function"], json!(1), "{stats}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_overwrite_load_in_place_keeps_no_node_whose_key_is_absent_from_the_load() -> TestResult
{
    // @step Given a code graph loaded with the File src/lib.rs and fn a
    let (_a_directory, root) = project(&[LIB_A])?;
    index(&root).await?;
    let (_b_directory, other) = project(&[LIB_B])?;
    index(&other).await?;
    let load = entities(&other).await?;

    // @step When the same database is Overwrite-loaded with the File src/lib.rs and fn b
    let mut lease = registry::write_code_graph(&root).await?;
    lease
        .database()
        .await?
        .load_entities_overwrite(&load)
        .await?;
    drop(lease);

    let (functions, exported, function_count) = code_graph_view(&root).await?;
    // @step Then a query of every function returns b only
    assert_eq!(functions, ["b"]);

    // @step And the export holds the File and b only
    assert_eq!(exported, file_and_b());

    // @step And the stats count 1 Function
    assert_eq!(function_count, json!(1));
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_import_in_overwrite_mode_replaces_a_code_graphs_definitions() -> TestResult {
    // @step Given a code graph holding fn a
    let (_a_directory, root) = project(&[LIB_A])?;
    index(&root).await?;

    // @step And a bundle exported from a code graph holding fn b
    let (b_directory, other) = project(&[LIB_B])?;
    index(&other).await?;
    let bundle = b_directory.path().join("b.astbundle");
    registry::read_code_graph(&other)
        .await?
        .export_bundle(&bundle, registry::AST_CODE_SCHEMA)
        .await?;

    // @step When the bundle is imported into the first graph in overwrite mode
    let mut lease = registry::write_code_graph(&root).await?;
    lease
        .database()
        .await?
        .import_bundle(&bundle, registry::AST_CODE_SCHEMA, "overwrite")
        .await?;
    drop(lease);

    let (functions, exported, function_count) = code_graph_view(&root).await?;
    // @step Then a query of every function returns b only
    assert_eq!(functions, ["b"]);

    // @step And the export holds the File and b only
    assert_eq!(exported, file_and_b());

    // @step And the stats count 1 Function
    assert_eq!(function_count, json!(1));
    Ok(())
}

fn learning(slug: &str, title: &str) -> String {
    json!({"type": "Learning", "data": {"slug": slug, "title": title, "content": title,
        "category": "pattern", "confidence": "high", "firstSeen": "2026-11-08T00:00:00",
        "lastSeen": "2026-11-08T00:00:00", "mentionCount": 1}})
    .to_string()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_bundle_import_in_overwrite_mode_replaces_a_learnings_graphs_learnings() -> TestResult {
    let directory = setup_temp_directory()?;

    // @step Given a learnings graph holding learning A
    let db = GraphDatabase::init(
        &directory.path().join("a-learnings.nano"),
        registry::LEARNINGS_SCHEMA,
    )
    .await?;
    db.load_jsonl(&learning("learning-a", "alpha learning"))
        .await?;

    // @step And a bundle exported from a learnings graph holding learning B
    let other = GraphDatabase::init(
        &directory.path().join("b-learnings.nano"),
        registry::LEARNINGS_SCHEMA,
    )
    .await?;
    other
        .load_jsonl(&learning("learning-b", "beta learning"))
        .await?;
    let bundle = directory.path().join("b.astbundle");
    other
        .export_bundle(&bundle, registry::LEARNINGS_SCHEMA)
        .await?;

    // @step When the bundle is imported into the first graph in overwrite mode
    db.import_bundle(&bundle, registry::LEARNINGS_SCHEMA, "overwrite")
        .await?;

    // @step Then learnings_search returns B and not A
    let search = parsed(&dispatch_learnings_search(&db, "learning", None, Some(50)).await)?;
    let titles: Vec<&str> = search["results"]
        .as_array()
        .ok_or_else(|| format!("no results: {search}"))?
        .iter()
        .filter_map(|row| row["title"].as_str())
        .collect();
    assert_eq!(titles, ["beta learning"], "{search}");

    // @step And the export holds learning B only
    assert_eq!(
        node_names(&db.export_all_entities().await?),
        [("Learning".to_string(), "beta learning".to_string())]
    );

    // @step And the stats count 1 Learning
    let stats = parsed(&dispatch_learnings_stats(&db).await)?;
    assert_eq!(stats["nodes"]["Learning"], json!(1), "{stats}");
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn indexing_again_without_reset_writes_over_the_existing_graph() -> TestResult {
    // @step Given a project that has been indexed
    let (_directory, root) = project(&[LIB_A])?;
    index(&root).await?;

    // @step And a marker file placed in the project's code graph directory
    let marker = registry::code_graph_path(&root).join("bug643-marker");
    std::fs::write(&marker, "kept unless the graph is deleted")?;

    // @step When ast_index runs for the project again without reset
    index(&root).await?;

    // @step Then the marker file is still in the code graph directory
    assert!(marker.is_file(), "{} was deleted", marker.display());

    // @step When ast_index runs for the project with reset
    let output = parsed(&dispatch_ast_index(&root, true, false).await)?;
    assert!(output.get("error").is_none(), "{output}");

    // @step Then the marker file is gone
    assert!(!marker.exists(), "{} survived a reset", marker.display());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn indexing_a_project_whose_source_files_were_all_removed_leaves_an_empty_graph() -> TestResult
{
    // @step Given a project whose src/lib.rs holds fn a has been indexed
    let (_directory, root) = project(&[LIB_A])?;
    index(&root).await?;

    // @step When src/lib.rs is deleted and ast_index runs for the project again without reset
    std::fs::remove_file(root.join(LIB_A.0))?;
    index(&root).await?;

    // @step Then the graph's export holds no node
    let graph = registry::read_code_graph(&root).await?;
    assert_eq!(node_names(&graph.export_all_entities().await?), []);

    // @step And ast_stats counts 0 File and 0 Function
    let stats = parsed(&dispatch_ast_stats(&graph).await)?;
    assert_eq!(stats["nodes"]["File"], json!(0), "{stats}");
    assert_eq!(stats["nodes"]["Function"], json!(0), "{stats}");
    Ok(())
}

/// A Python file whose `count` functions each call the next one, preceded
/// by `head`.
fn call_chain(head: &str, count: usize) -> String {
    let mut text = head.to_string();
    for i in 0..count {
        let body = if i + 1 < count {
            format!("f{}()", i + 1)
        } else {
            "0".to_string()
        };
        text.push_str(&format!("def f{i}():\n    return {body}\n\n\n"));
    }
    text
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_in_place_index_that_renumbers_thousands_of_rows_succeeds() -> TestResult {
    // @step Given a project whose Python file holds 5,000 functions that each call the next has been indexed
    let (_directory, root) = project(&[("chain.py", &call_chain("", 5_000))])?;
    index(&root).await?;

    // @step When a function that calls the first is added at the top of the file and ast_index runs for the project again without reset
    std::fs::write(
        root.join("chain.py"),
        call_chain("def start():\n    return f0()\n\n\n", 5_000),
    )?;
    let output = parsed(&dispatch_ast_index(&root, false, false).await)?;

    // @step Then the index succeeds
    assert!(output.get("error").is_none(), "{output}");

    // @step And the graph holds 5,001 functions and 5,000 Calls edges
    let graph = registry::read_code_graph(&root).await?;
    let stats = parsed(&dispatch_ast_stats(&graph).await)?;
    assert_eq!(stats["nodes"]["Function"], json!(5_001), "{stats}");
    assert_eq!(stats["edges"]["Calls"], json!(5_000), "{stats}");
    let all = graph.export_all_entities().await?;
    let calls = all
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Calls"))
        .count();
    assert_eq!(calls, 5_000);
    Ok(())
}
