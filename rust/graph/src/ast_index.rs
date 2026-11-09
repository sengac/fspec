#![allow(clippy::await_holding_lock)]
//! AST Index Dispatch
//!
//! Handles the `ast_index` action: walks the project directory, extracts
//! the facts of its source files, resolves every edge and loads the
//! entities into the AST code graph.
//!
//! Supports two modes (KGRAPH-076), which build the same graph:
//! - **Full** (default): parses every file and writes the graph.
//! - **Incremental**: reuses the cached facts of every file whose path and
//!   contentHash are unchanged, and writes nothing when the entities are
//!   those the last index loaded.
//!
//! Extracted from `ast_dispatch.rs` for file size compliance.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::ast_pipeline::facts::FactCache;
use crate::dispatch_helpers::{graph_stats, stamp, with_root};
use crate::graph_entities::entities_to_jsonl;

/// Index (or re-index) `root` into its own code graph (KGRAPH-073).
///
/// Walks `root`, extracts the facts of every source file in one pass per
/// file, resolves every edge from the facts of the whole tree (KGRAPH-074,
/// KGRAPH-075), adds dependencies from the package manifests, and loads everything
/// into the code graph at `<root>/.fspec/graph/ast-code.nano`. The files
/// indexed depend only on `root`: `.gitignore` files at and below it apply,
/// ignore files above it do not. Every result names the canonical root.
///
/// The write lease is taken first: in this process the run waits for other
/// writers and running readers; when another process holds the graph's
/// writer lock the run fails without writing.
///
/// Each source file is read once; the SHA-256 of its bytes is its
/// contentHash, stored on its File node. An incremental index reuses the
/// facts this process cached for every file with the same path and
/// contentHash and parses the rest; a full index parses every file. Every
/// index then resolves the whole tree, so both build the same entities.
/// The graph is written with one Overwrite load, except that an incremental
/// index writes nothing when its entities hash to the record of the last
/// load (any other write deletes the record). When `reset` is `true` the
/// on-disk database and the root's fact cache are deleted first (needed
/// after a schema change).
pub async fn dispatch_ast_index(root: &Path, reset: bool, incremental: bool) -> String {
    let root = match super::registry::canonical_root(root) {
        Ok(root) => root,
        Err(e) => {
            return serde_json::json!({ "action": "ast_index", "error": e }).to_string();
        }
    };
    let output = index_root(&root, reset, incremental).await;
    with_root(output, &root)
}

fn error(message: String) -> String {
    serde_json::json!({ "action": "ast_index", "error": message }).to_string()
}

async fn index_root(root: &Path, reset: bool, incremental: bool) -> String {
    let mut lease = match super::registry::write_code_graph(root).await {
        Ok(lease) => lease,
        Err(e) => return error(e),
    };
    if reset {
        match lease.reset() {
            Ok(deleted) => tracing::info!(deleted, "AST graph reset: on-disk data deleted"),
            Err(e) => return error(format!("Failed to reset graph: {e}")),
        }
    }
    let cache = if incremental {
        match lease.fact_cache() {
            Ok(cache) => cache,
            Err(e) => return error(e),
        }
    } else {
        FactCache::new()
    };
    let indexed = super::ast_pipeline::index_tree_cached(
        root,
        true,
        super::ast_pipeline::facts::index_threads(),
        &cache,
    );
    drop(cache);
    if let Err(e) = lease.store_fact_cache(indexed.facts) {
        return error(e);
    }
    let jsonl = entities_to_jsonl(&indexed.entities);
    let hash = format!("{:x}", Sha256::digest(jsonl.as_bytes()));

    let db = match lease.database().await {
        Ok(db) => db,
        Err(e) => return error(e),
    };
    let unchanged = if incremental {
        match db.index_record() {
            Ok(record) => record.as_deref() == Some(hash.as_str()),
            Err(e) => return error(e),
        }
    } else {
        false
    };
    if !unchanged {
        if let Err(e) = db.load_jsonl_overwrite(&jsonl).await {
            return error(format!("Failed to load entities into graph: {e}"));
        }
        if let Err(e) = db.write_index_record(&hash) {
            return error(e);
        }
        tracing::info!(
            loaded = indexed.entities.len(),
            "AST index complete — entities loaded into graph"
        );
    }
    // KGRAPH-078: the graph holds this index's entities (just written, or
    // equal to the record), so it records their unresolved counts.
    let unresolved = serde_json::json!(indexed.unresolved);
    if let Err(e) = db.write_unresolved_record(&unresolved) {
        return error(e);
    }
    let mut extra = serde_json::Map::new();
    extra.insert("written".into(), (!unchanged).into());
    extra.insert("files_parsed".into(), indexed.files_parsed.into());
    extra.insert("files_reused".into(), indexed.files_reused.into());
    extra.insert("files_skipped".into(), serde_json::json!(indexed.skipped));
    extra.insert("files_failed".into(), serde_json::json!(indexed.failed));
    extra.insert(
        "dependencies_failed".into(),
        serde_json::json!(indexed.dependency_failures),
    );
    extra.insert("unresolved".into(), unresolved);
    let loaded = if unchanged { 0 } else { indexed.entities.len() };
    build_index_result(db, loaded, incremental, extra)
}

/// The JSON result: graph stats plus the index's own fields, stamped with
/// the graph version the index wrote or found (KGRAPH-077).
fn build_index_result(
    db: &super::database::GraphDatabase,
    loaded: usize,
    incremental: bool,
    extra: serde_json::Map<String, serde_json::Value>,
) -> String {
    let mut result = graph_stats(db, "ast_index");
    if let Some(obj) = result.as_object_mut() {
        obj.insert("entities_loaded".into(), loaded.into());
        if incremental {
            obj.insert("incremental".into(), true.into());
        }
        obj.extend(extra);
    }
    stamp(db, result)
}
