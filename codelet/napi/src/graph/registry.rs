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
async fn init_graph(name: &str) -> Result<GraphDatabase, String> {
    let (db_path, schema) = resolve_graph_config(name)?;

    info!(?db_path, name, "initializing graph database");

    GraphDatabase::open_or_init(&db_path, schema).await
}

/// Resolve the path and schema for a named graph.
fn resolve_graph_config(name: &str) -> Result<(PathBuf, &'static str), String> {
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
