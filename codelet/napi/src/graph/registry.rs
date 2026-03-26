//! Graph Database Registry — Named Instance Management
//!
//! Manages named `GraphDatabase` singletons for the dual-graph architecture:
//!   - `"ast-code"` — the AST structural code graph (project-scoped)
//!   - `"learnings"` — the Residue-methodology learnings graph (global)
//!
//! Each graph is lazily initialized on first access.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{info, warn};

use super::database::GraphDatabase;

lazy_static::lazy_static! {
    /// Global registry of named graph database instances.
    static ref REGISTRY: Mutex<HashMap<String, GraphDatabase>> = Mutex::new(HashMap::new());
}

/// Well-known graph names.
pub const AST_CODE_GRAPH: &str = "ast-code";
pub const LEARNINGS_GRAPH: &str = "learnings";

/// Bundled schemas, compiled into the binary.
const AST_CODE_SCHEMA: &str = include_str!("../../schemas/ast-code.pg");
const LEARNINGS_SCHEMA: &str = include_str!("../../schemas/learnings.pg");

/// Get a graph database by name, initializing it if needed.
///
/// For `"ast-code"`: stored at `<project>/.fspec/graph/ast-code.nano/` (project-scoped).
/// For `"learnings"`: stored at `~/.fspec/graph/learnings.nano/` (global).
pub async fn get_graph(name: &str) -> Result<GraphDatabase, String> {
    // Fast path: check if already initialized
    {
        let guard = REGISTRY
            .lock()
            .map_err(|e| format!("Graph registry lock poisoned: {e}"))?;
        if let Some(db) = guard.get(name) {
            return Ok(db.clone());
        }
    }

    // Slow path: initialize the graph
    let db = init_graph(name).await?;

    // Store in registry
    let mut guard = REGISTRY
        .lock()
        .map_err(|e| format!("Graph registry lock poisoned: {e}"))?;
    guard.insert(name.to_string(), db.clone());

    Ok(db)
}

/// Check if a named graph is currently initialized in the registry.
pub fn is_graph_initialized(name: &str) -> bool {
    match REGISTRY.lock() {
        Ok(guard) => guard.contains_key(name),
        Err(e) => {
            warn!("Graph registry lock poisoned in is_graph_initialized: {e}");
            false
        }
    }
}

/// Reset a specific named graph, removing it from the registry.
///
/// Available for use when a specific graph needs to be re-initialized
/// (e.g., after schema migration or data directory change for a single graph).
pub fn reset_graph(name: &str) {
    match REGISTRY.lock() {
        Ok(mut guard) => {
            if guard.remove(name).is_some() {
                info!(name, "reset graph database from registry");
            }
        }
        Err(e) => {
            warn!("Failed to reset graph '{name}' (lock poisoned): {e}");
        }
    }
}

/// Delete the on-disk data for a named graph AND remove it from the in-memory registry.
///
/// This is the "nuclear reset" for when the schema has changed and the existing
/// database is incompatible. Deletes the `.nano/` directory from disk and clears
/// the in-memory singleton so the next `get_graph()` call will re-initialize
/// with the compiled (current) schema.
///
/// Returns `Ok(true)` if data was deleted, `Ok(false)` if no data existed on disk.
pub fn delete_graph_data(name: &str, db_path: &std::path::Path) -> Result<bool, String> {
    // 1. Remove from in-memory registry first
    reset_graph(name);

    // 2. Delete on-disk data
    if db_path.exists() {
        std::fs::remove_dir_all(db_path)
            .map_err(|e| format!("Failed to delete graph data at {}: {e}", db_path.display()))?;
        info!(?db_path, name, "deleted on-disk graph data");
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Insert a graph database into the registry (for testing).
#[cfg(test)]
pub fn insert_graph_for_test(name: &str, db: super::database::GraphDatabase) {
    let mut guard = REGISTRY
        .lock()
        .expect("Graph registry lock poisoned in test");
    guard.insert(name.to_string(), db);
}

/// Reset ALL graph databases in the registry.
///
/// Called when the data directory changes.
pub fn reset_all_graphs() {
    match REGISTRY.lock() {
        Ok(mut guard) => {
            if !guard.is_empty() {
                info!(count = guard.len(), "resetting all graph databases");
            }
            guard.clear();
        }
        Err(e) => {
            warn!("Failed to reset all graphs (lock poisoned): {e}");
        }
    }
}

/// Close all graph databases cleanly.
///
/// Should be called on process exit.
pub fn close_all_graphs() {
    match REGISTRY.lock() {
        Ok(mut guard) => {
            if !guard.is_empty() {
                info!(count = guard.len(), "closing all graph databases");
            }
            guard.clear(); // Drop triggers Database cleanup
        }
        Err(e) => {
            warn!("Failed to close all graphs (lock poisoned): {e}");
        }
    }
}

/// Initialize a specific named graph database.
///
/// Uses schema hash validation: if the on-disk schema doesn't match the
/// compiled schema, returns an actionable error instead of silently opening
/// with stale schema.
async fn init_graph(name: &str) -> Result<GraphDatabase, String> {
    let (db_path, schema) = resolve_graph_config(name)?;

    info!(?db_path, name, "initializing graph database");

    GraphDatabase::open_or_init_with_schema_check(&db_path, schema).await
}

/// Resolve the path and schema for a named graph (public for reset operations).
pub fn resolve_graph_config(name: &str) -> Result<(PathBuf, &'static str), String> {
    match name {
        AST_CODE_GRAPH => {
            let project_dir = resolve_project_dir()?;
            let db_path = project_dir.join(".fspec/graph/ast-code.nano");
            Ok((db_path, AST_CODE_SCHEMA))
        }
        LEARNINGS_GRAPH => {
            let data_dir = codelet_common::get_data_dir()?;
            let db_path = data_dir.join("graph/learnings.nano");
            Ok((db_path, LEARNINGS_SCHEMA))
        }
        _ => Err(format!("Unknown graph name: '{name}'. Known: {AST_CODE_GRAPH}, {LEARNINGS_GRAPH}")),
    }
}

/// Resolve the current project directory for project-scoped graphs.
fn resolve_project_dir() -> Result<PathBuf, String> {
    std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))
}
