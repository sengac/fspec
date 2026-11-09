// Feature: spec/features/incremental-reindexing.feature
//
// Incremental Re-indexing
// Tests ast_index's incremental mode (KGRAPH-076): the facts of files
// whose content hash is unchanged are reused and only the other files are
// parsed; every edge comes from the facts of the whole tree, so changed and
// new files get fresh entities, unchanged files keep identical entities,
// and deleted files' entities are removed. Every test runs the real
// `ast_index`.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Helper: create a temp project with source files and return their paths.
fn create_test_project(dir: &Path) -> Vec<std::path::PathBuf> {
    let src = dir.join("src");
    std::fs::create_dir_all(&src).expect("create src");

    let main_ts = src.join("main.ts");
    std::fs::write(
        &main_ts,
        "export function handleRequest(req: any, res: any) {\n  return res.json({});\n}\n",
    )
    .expect("write main.ts");

    let utils_ts = src.join("utils.ts");
    std::fs::write(
        &utils_ts,
        "export function parseData(data: string) {\n  return JSON.parse(data);\n}\n",
    )
    .expect("write utils.ts");

    let helper_ts = src.join("helper.ts");
    std::fs::write(
        &helper_ts,
        "export function formatOutput(msg: string): string {\n  return msg.trim();\n}\n",
    )
    .expect("write helper.ts");

    vec![main_ts, utils_ts, helper_ts]
}

/// A temp directory holding the test project; returns it (keep it alive)
/// and the project's canonical root.
fn temp_project() -> (tempfile::TempDir, PathBuf) {
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let root = temp_dir.path().canonicalize().expect("canonical root");
    create_test_project(&root);
    (temp_dir, root)
}

/// Run `ast_index` on `root` and return its JSON result.
async fn run_index(root: &Path, incremental: bool) -> Value {
    let output = dispatch_ast_index(root, false, incremental).await;
    let value: Value = serde_json::from_str(&output).expect("ast_index result is JSON");
    assert!(value.get("error").is_none(), "ast_index failed: {output}");
    value
}

/// Every entity of `root`'s code graph.
async fn graph_entities(root: &Path) -> Vec<GraphEntity> {
    let graph = registry::read_code_graph(root).await.expect("open graph");
    graph.export_all_entities().await.expect("export")
}

/// The (node type, qualifiedName or path) of every node, sorted.
fn node_names(entities: &[GraphEntity]) -> Vec<(String, String)> {
    let mut names: Vec<(String, String)> = entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type,
                properties,
                ..
            } => {
                let key = if node_type == "File" {
                    "path"
                } else {
                    "qualifiedName"
                };
                let name = properties.get(key)?.as_str()?.to_string();
                Some((node_type.clone(), name))
            }
            _ => None,
        })
        .collect();
    names.sort();
    names
}

/// The entities belonging to the file at `path`: its File node, the nodes
/// whose qualifiedName starts with `path::` and edges touching any of them.
fn entities_of(entities: &[GraphEntity], path: &str) -> Vec<String> {
    let prefix = format!("{path}::");
    let owned = |slug: &str| slug == path || slug.starts_with(&prefix);
    let mut found: Vec<String> = entities
        .iter()
        .filter(|e| match e {
            GraphEntity::Node { slug, .. } => owned(slug),
            GraphEntity::Edge {
                from_slug, to_slug, ..
            } => owned(from_slug) || owned(to_slug),
        })
        .map(|e| format!("{e:?}"))
        .collect();
    found.sort();
    found
}

#[path = "ast_incremental_test/incremental_adds_new_file.rs"]
mod incremental_adds_new_file;
#[path = "ast_incremental_test/unchanged_modified_and_deleted_files.rs"]
mod unchanged_modified_and_deleted_files;
