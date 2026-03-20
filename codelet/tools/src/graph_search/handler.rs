//! GraphSearch Tool — Handler Registration
//!
//! Per-session handler map following the SessionSearch pattern.
//! The concrete handler is registered by codelet-napi at session start.

use super::types::GraphSearchAction;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use uuid::Uuid;

/// Handler function type: takes an action + session_id, returns a JSON string result.
pub type GraphSearchHandler =
    Arc<dyn Fn(GraphSearchAction, Uuid) -> String + Send + Sync>;

/// Global handler map: one handler per session.
static GRAPH_SEARCH_HANDLERS: Lazy<RwLock<HashMap<Uuid, GraphSearchHandler>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Register (or unregister) a handler for a session.
pub fn set_graph_search_handler(session_id: Uuid, handler: Option<GraphSearchHandler>) {
    let mut map = match GRAPH_SEARCH_HANDLERS.write() {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("GraphSearch handler map poisoned on write: {e}");
            return;
        }
    };
    match handler {
        Some(h) => {
            map.insert(session_id, h);
        }
        None => {
            map.remove(&session_id);
        }
    }
}

/// Check if a handler exists for a session.
pub fn has_graph_search_handler(session_id: Uuid) -> bool {
    GRAPH_SEARCH_HANDLERS
        .read()
        .map(|map| map.contains_key(&session_id))
        .unwrap_or(false)
}

/// Execute a GraphSearch action for a session.
///
/// Looks up the handler and calls it. Returns a descriptive error string
/// (not a panic) if no handler is registered.
pub fn execute_graph_search(session_id: Uuid, action: GraphSearchAction) -> String {
    let map = match GRAPH_SEARCH_HANDLERS.read() {
        Ok(m) => m,
        Err(_) => return r#"{"error":"GraphSearch handler map poisoned"}"#.to_string(),
    };

    match map.get(&session_id) {
        Some(handler) => handler(action, session_id),
        None => r#"{"error":"No handler registered for this session. GraphSearch is not available."}"#.to_string(),
    }
}

/// Remove all handlers (used in test cleanup).
pub fn clear_all_graph_search_handlers() {
    if let Ok(mut map) = GRAPH_SEARCH_HANDLERS.write() {
        map.clear();
    }
}
