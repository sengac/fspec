//! Watermark State for Incremental Indexing
//!
//! Tracks per-session indexing progress (last indexed turn) for
//! incremental graph updates. State is persisted to index-state.json.
//!
//! Extracted from merge.rs to keep files under 300 lines.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

/// Watermark state for incremental indexing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexState {
    #[serde(rename = "lastRunAt")]
    pub last_run_at: String,
    #[serde(rename = "schemaVersion", default = "default_schema_version")]
    pub schema_version: String,
    pub sessions: HashMap<String, SessionWatermark>,
}

fn default_schema_version() -> String {
    "1".to_string()
}

impl Default for IndexState {
    fn default() -> Self {
        Self {
            last_run_at: String::new(),
            schema_version: "1".to_string(),
            sessions: HashMap::new(),
        }
    }
}

/// Per-session watermark tracking the last indexed turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionWatermark {
    #[serde(rename = "lastIndexedTurn")]
    pub last_indexed_turn: u32,
    #[serde(rename = "lastIndexedAt")]
    pub last_indexed_at: String,
}

/// Read index-state.json from the graph data directory.
///
/// Returns defaults if the file doesn't exist, is unreadable, or contains invalid JSON.
pub fn read_index_state(graph_dir: &Path) -> IndexState {
    let path = graph_dir.join("index-state.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => match serde_json::from_str(&content) {
            Ok(state) => state,
            Err(e) => {
                warn!("Failed to parse index-state.json: {e}, using defaults");
                IndexState::default()
            }
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => IndexState::default(),
        Err(e) => {
            warn!("Failed to read index-state.json: {e}, using defaults");
            IndexState::default()
        }
    }
}

/// Write index-state.json atomically (temp file + rename).
pub fn write_index_state(graph_dir: &Path, state: &IndexState) -> Result<(), String> {
    let path = graph_dir.join("index-state.json");
    let temp_path = graph_dir.join("index-state.json.tmp");

    let content = serde_json::to_string_pretty(state)
        .map_err(|e| format!("Failed to serialize index state: {e}"))?;

    std::fs::write(&temp_path, &content)
        .map_err(|e| format!("Failed to write temp index state: {e}"))?;

    std::fs::rename(&temp_path, &path)
        .map_err(|e| format!("Failed to rename index state: {e}"))?;

    info!("index-state.json updated atomically");
    Ok(())
}

/// Update watermark for a session after successful batch processing.
pub fn update_session_watermark(
    state: &mut IndexState,
    session_id: &str,
    last_indexed_turn: u32,
    now: &str,
) {
    state.last_run_at = now.to_string();
    state.sessions.insert(
        session_id.to_string(),
        SessionWatermark {
            last_indexed_turn,
            last_indexed_at: now.to_string(),
        },
    );
}
