//! Shared fixture helpers for the code graph tests of KGRAPH-074/075: a
//! temp project, an index of it, and the graph's entities.
//!
//! Each test binary includes this module via `pub mod code_graph_fixture;`
//! and uses a subset of the helpers; the module is `pub` so the unused ones
//! do not trip `dead_code`.

use std::path::{Path, PathBuf};

use codelet_graph::ast_index::dispatch_ast_index;
use codelet_graph::graph_entities::GraphEntity;
use codelet_graph::registry;
use codelet_test_helpers::{setup_temp_directory, TestError};
use serde_json::{Map, Value};

/// A temp project holding `files` (relative path, text); returns the temp
/// directory (keep it alive) and the project's canonical root.
pub fn project(files: &[(&str, &str)]) -> Result<(tempfile::TempDir, PathBuf), TestError> {
    let directory = setup_temp_directory()?;
    let root = directory.path().join("project");
    std::fs::create_dir_all(&root)?;
    for (path, text) in files {
        let full = root.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(full, text)?;
    }
    let root = root.canonicalize()?;
    Ok((directory, root))
}

/// A one-crate Cargo manifest, so the files reached from src/lib.rs are in
/// a module tree.
pub const CRATE_MANIFEST: (&str, &str) = ("Cargo.toml", "[package]\nname = \"fixture\"\n");

/// Run a full `ast_index` of `root` and return its result.
pub async fn index(root: &Path) -> Result<Value, TestError> {
    let output: Value = serde_json::from_str(&dispatch_ast_index(root, false, false).await)?;
    if let Some(error) = output.get("error") {
        return Err(format!("ast_index failed: {error}").into());
    }
    Ok(output)
}

/// Every entity of `root`'s graph.
pub async fn entities(root: &Path) -> Result<Vec<GraphEntity>, TestError> {
    let graph = registry::read_code_graph(root).await?;
    Ok(graph.export_all_entities().await?)
}

/// (slug, properties) of every node of `node_type`.
pub fn nodes(entities: &[GraphEntity], node_type: &str) -> Vec<(String, Map<String, Value>)> {
    entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Node {
                node_type: t,
                slug,
                properties,
            } if t == node_type => Some((slug.clone(), properties.clone())),
            _ => None,
        })
        .collect()
}

/// (from, to, properties) of every edge of `edge_type`.
pub fn edges(
    entities: &[GraphEntity],
    edge_type: &str,
) -> Vec<(String, String, Map<String, Value>)> {
    entities
        .iter()
        .filter_map(|e| match e {
            GraphEntity::Edge {
                edge_type: t,
                from_slug,
                to_slug,
                properties,
            } if t == edge_type => Some((from_slug.clone(), to_slug.clone(), properties.clone())),
            _ => None,
        })
        .collect()
}

/// The slug of the one node of `node_type` whose qualifiedName is `name`.
pub fn slug_of(
    entities: &[GraphEntity],
    node_type: &str,
    qualified_name: &str,
) -> Result<String, TestError> {
    let found: Vec<String> = nodes(entities, node_type)
        .into_iter()
        .filter(|(_, p)| p.get("qualifiedName").and_then(Value::as_str) == Some(qualified_name))
        .map(|(slug, _)| slug)
        .collect();
    match found.as_slice() {
        [one] => Ok(one.clone()),
        _ => Err(format!(
            "{} {node_type} nodes named {qualified_name}: {found:?}",
            found.len()
        )
        .into()),
    }
}

/// The qualifiedNames the Calls edges from `caller` lead to, sorted.
pub fn callees_of(entities: &[GraphEntity], caller: &str) -> Vec<String> {
    let names: std::collections::BTreeMap<String, String> = nodes(entities, "Function")
        .into_iter()
        .filter_map(|(slug, p)| Some((slug, p.get("qualifiedName")?.as_str()?.to_string())))
        .collect();
    let mut found: Vec<String> = edges(entities, "Calls")
        .into_iter()
        .filter(|(from, _, _)| from == caller)
        .filter_map(|(_, to, _)| names.get(&to).cloned())
        .collect();
    found.sort();
    found
}

/// The count of unresolved references of `kind` with `reason` in an index
/// result (0 when absent).
pub fn unresolved(result: &Value, kind: &str, reason: &str) -> u64 {
    result["unresolved"][kind][reason].as_u64().unwrap_or(0)
}
