//! Graph Database Module — Nanograph Integration
//!
//! Provides an embedded nanograph property graph database for building
//! a knowledge graph across agent sessions. The database is lazily
//! initialized on first use and stored at `~/.fspec/graph/agent-memory.nano/`.
//!
//! Follows the same singleton pattern as `persistence::MESSAGE_STORE`.
//!
//! ## Usage
//! ```rust,ignore
//! // Ensure DB is ready (auto-inits on first call)
//! graph::ensure_graph_db().await?;
//!
//! // Get stats
//! let stats = graph::graph_db_stats().await?;
//!
//! // Reset singleton (called on data directory change)
//! graph::reset_graph_db();
//! ```

use nanograph::result::RunResult;
use nanograph::store::database::Database;
use std::path::PathBuf;
use std::sync::Mutex;
use tracing::{info, warn};

/// The bundled agent-memory schema, compiled into the binary.
const AGENT_MEMORY_SCHEMA: &str = include_str!("../../schemas/agent-memory.pg");

/// Relative path from data directory to graph database.
const GRAPH_DB_RELATIVE_PATH: &str = "graph/agent-memory.nano";

lazy_static::lazy_static! {
    /// Global singleton for the nanograph database.
    /// `None` means not yet initialized (or reset after directory change).
    static ref GRAPH_DB: Mutex<Option<Database>> = Mutex::new(None);
}

/// Derive the full database path from the global data directory.
fn graph_db_path() -> Result<PathBuf, String> {
    let data_dir = codelet_common::get_data_dir()?;
    Ok(data_dir.join(GRAPH_DB_RELATIVE_PATH))
}

/// Ensure the graph database is open and ready.
///
/// If the singleton is `None`:
/// - If the `.nano` directory exists on disk → `Database::open()`
/// - If it doesn't exist → `Database::init()` with the bundled schema
///
/// This is an async function because nanograph's init/open are async
/// (they use tokio for Lance operations).
pub async fn ensure_graph_db() -> Result<(), String> {
    // Fast path: already initialized
    {
        let guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
        if guard.is_some() {
            return Ok(());
        }
    }

    let db_path = graph_db_path()?;
    info!(?db_path, "ensuring graph database");

    let db = if db_path.exists() && db_path.join("schema.ir.json").exists() {
        // Existing database — open it
        info!("opening existing graph database");
        Database::open(&db_path)
            .await
            .map_err(|e| format!("Failed to open graph DB: {e}"))?
    } else {
        // New database — create directory structure and init with schema
        info!("initializing new graph database");
        let parent = db_path.parent().ok_or_else(|| "Invalid graph DB path".to_string())?;
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create graph directory: {e}"))?;
        Database::init(&db_path, AGENT_MEMORY_SCHEMA)
            .await
            .map_err(|e| format!("Failed to init graph DB: {e}"))?
    };

    // Store in singleton
    let mut guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
    *guard = Some(db);

    info!("graph database ready");
    Ok(())
}

/// Check if the graph database singleton is currently initialized.
pub fn is_graph_initialized() -> bool {
    match GRAPH_DB.lock() {
        Ok(guard) => guard.is_some(),
        Err(e) => {
            warn!("Graph DB lock poisoned in is_graph_initialized: {e}");
            false
        }
    }
}

/// Reset the graph database singleton to `None`.
///
/// Called when the data directory changes (via `set_data_directory()`).
/// The next call to `ensure_graph_db()` will re-initialize from the new path.
pub fn reset_graph_db() {
    match GRAPH_DB.lock() {
        Ok(mut guard) => {
            if guard.is_some() {
                info!("resetting graph database singleton");
            }
            *guard = None;
        }
        Err(e) => {
            warn!("Failed to reset graph DB (lock poisoned): {e}");
        }
    }
}

/// Get stats about the graph database.
///
/// Returns a JSON string with node/edge type counts (actual row counts from storage).
pub async fn graph_db_stats() -> Result<String, String> {
    ensure_graph_db().await?;

    let guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
    let db = guard.as_ref().ok_or("Graph DB not initialized")?;

    let storage = db.snapshot();
    let mut stats = serde_json::Map::new();

    // Count actual node rows per type from storage segments
    let mut node_types = serde_json::Map::new();
    for (name, _nt) in &db.catalog().node_types {
        let count: usize = storage
            .node_segments
            .get(name.as_str())
            .map(|seg| seg.batches.iter().map(|b| b.num_rows()).sum())
            .unwrap_or(0);
        node_types.insert(name.clone(), serde_json::Value::Number(count.into()));
    }
    stats.insert("nodes".to_string(), serde_json::Value::Object(node_types));

    // Count actual edge rows per type from storage segments
    let mut edge_types = serde_json::Map::new();
    for (name, _et) in &db.catalog().edge_types {
        let count: usize = storage
            .edge_segments
            .get(name.as_str())
            .map(|seg| seg.batches.iter().map(|b| b.num_rows()).sum())
            .unwrap_or(0);
        edge_types.insert(name.clone(), serde_json::Value::Number(count.into()));
    }
    stats.insert("edges".to_string(), serde_json::Value::Object(edge_types));

    serde_json::to_string_pretty(&serde_json::Value::Object(stats))
        .map_err(|e| format!("Failed to serialize stats: {e}"))
}

/// Load JSONL data into the graph database using merge mode.
///
/// Entities are converted to JSONL via `merge::entities_to_jsonl` and loaded
/// with nanograph's default mode (Merge when schema has @key properties).
pub async fn graph_db_load_jsonl(jsonl: &str) -> Result<(), String> {
    ensure_graph_db().await?;

    // Clone the Database handle (Arc-wrapped, cheap) to avoid holding the Mutex across await
    let db = {
        let guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
        guard.as_ref().ok_or("Graph DB not initialized")?.clone()
    };

    if jsonl.trim().is_empty() {
        return Ok(());
    }

    db.load(jsonl)
        .await
        .map_err(|e| format!("Failed to load JSONL into graph: {e}"))?;

    info!(lines = jsonl.lines().count(), "loaded JSONL into graph database");
    Ok(())
}

/// Run a named query against the graph database.
///
/// Returns the result as a serde_json::Value (array of row objects).
pub async fn graph_db_query(
    query_source: &str,
    query_name: &str,
    params: Option<&serde_json::Value>,
) -> Result<serde_json::Value, String> {
    ensure_graph_db().await?;

    // Clone the Database handle (Arc-wrapped, cheap) to avoid holding the Mutex across await
    let db = {
        let guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
        guard.as_ref().ok_or("Graph DB not initialized")?.clone()
    };

    let result = db
        .run_json(
            query_source,
            query_name,
            params,
            nanograph::query_input::JsonParamMode::Standard,
        )
        .await
        .map_err(|e| format!("Graph query failed: {e}"))?;

    match result {
        RunResult::Query(qr) => Ok(qr.to_rust_json()),
        RunResult::Mutation(mr) => Ok(serde_json::json!({
            "affected_nodes": mr.affected_nodes,
            "affected_edges": mr.affected_edges,
        })),
    }
}

/// Describe the graph schema.
///
/// Returns a human-readable description of all node types, edge types,
/// and their properties.
pub async fn graph_describe_schema() -> Result<String, String> {
    ensure_graph_db().await?;

    let guard = GRAPH_DB.lock().map_err(|e| format!("Graph DB lock poisoned: {e}"))?;
    let db = guard.as_ref().ok_or("Graph DB not initialized")?;

    let catalog = db.catalog();
    let mut description = String::new();

    description.push_str("=== Node Types ===\n");
    for (name, nt) in &catalog.node_types {
        description.push_str(&format!("  {}\n", name));
        for (prop_name, prop_type) in &nt.properties {
            description.push_str(&format!("    - {}: {}\n", prop_name, prop_type.display_name()));
        }
    }

    description.push_str("\n=== Edge Types ===\n");
    for (name, et) in &catalog.edge_types {
        description.push_str(&format!("  {} ({} -> {})\n", name, et.from_type, et.to_type));
        for (prop_name, prop_type) in &et.properties {
            description.push_str(&format!("    - {}: {}\n", prop_name, prop_type.display_name()));
        }
    }

    Ok(description)
}

/// Close the graph database cleanly.
///
/// Should be called on process exit to avoid Lance corruption.
/// After this call, `is_graph_initialized()` returns `false`.
pub fn close_graph_db() {
    match GRAPH_DB.lock() {
        Ok(mut guard) => {
            if guard.is_some() {
                info!("closing graph database");
            }
            *guard = None; // Drop triggers Database cleanup
        }
        Err(e) => {
            warn!("Failed to close graph DB (lock poisoned): {e}");
        }
    }
}

pub mod compaction;
pub mod deepsearch_integration;
pub mod dispatch;
pub mod entity_pipeline;
pub mod extractors;
pub mod indexing;
pub mod llm_caller;
pub mod llm_extraction;
pub mod llm_validation;
pub mod merge;
pub mod queries;
pub mod session_scanner;
pub mod watermark;

#[cfg(test)]
mod tests;
