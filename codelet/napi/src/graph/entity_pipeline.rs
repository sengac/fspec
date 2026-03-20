//! Entity Pipeline — Global queue and extraction integration
//!
//! Manages the pending entity queue for structural extractors,
//! threshold-based auto-flushing, and tool-call extraction integration.
//!
//! Extracted from graph_search_handler.rs to keep files under 300 lines.

use super::extractors::{EntityQueue, GraphEntity};
use crate::graph;

lazy_static::lazy_static! {
    /// Global pending entity queue — populated by structural extractors,
    /// flushed on index action or threshold.
    static ref PENDING_ENTITIES: std::sync::Mutex<EntityQueue> =
        std::sync::Mutex::new(EntityQueue::new(50));
}

/// Flush any remaining pending entities to the graph database.
///
/// Called at the end of each agent turn (on StreamEvent::Done) to ensure
/// entities below the auto-flush threshold still get persisted.
pub fn flush_pending_entities() {
    if !graph::is_graph_initialized() {
        return;
    }

    let entities = {
        let mut queue = match PENDING_ENTITIES.lock() {
            Ok(q) => q,
            Err(e) => {
                tracing::warn!("Entity queue lock poisoned on flush: {e}");
                return;
            }
        };
        queue.flush()
    };

    if entities.is_empty() {
        return;
    }

    let jsonl = graph::merge::entities_to_jsonl(&entities);
    let count = entities.len();
    // Block on the async flush — Database futures are !Send so can't use tokio::spawn
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async move {
            match graph::graph_db_load_jsonl(&jsonl).await {
                Ok(()) => tracing::info!(count, "flushed remaining entities to graph on turn done"),
                Err(e) => tracing::warn!("failed to flush entities on turn done: {e}"),
            }
        })
    });
}

/// Extract entities from a tool call and queue them for batch loading.
///
/// This is the integration point between the tool execution pipeline and the
/// graph database. It runs structural extractors on the tool call data and
/// queues the resulting entities for batch loading.
pub fn extract_and_queue_from_tool_call(
    tool_name: &str,
    tool_args: &serde_json::Value,
    session_slug: &str,
    turn_index: u32,
) {
    // Skip if graph module is not initialized (avoids unnecessary work)
    if !graph::is_graph_initialized() {
        return;
    }

    let entities = match tool_name {
        "Write" | "Edit" => {
            if let Some(file_path) = tool_args.get("file_path").and_then(|v| v.as_str()) {
                graph::extractors::extract_from_file_operation(
                    tool_name, file_path, session_slug, turn_index,
                )
            } else {
                return;
            }
        }
        "Fspec" => {
            if let Some(command) = tool_args.get("command").and_then(|v| v.as_str()) {
                // Extract work unit ID from args
                let work_unit_id = tool_args
                    .get("args")
                    .and_then(|a| a.get("_"))
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let title = tool_args
                    .get("args")
                    .and_then(|a| a.get("_"))
                    .and_then(|arr| arr.as_array())
                    .and_then(|arr| arr.get(1))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                graph::extractors::extract_from_fspec_command(
                    command, work_unit_id, title, session_slug,
                )
            } else {
                return;
            }
        }
        _ => return,
    };

    if entities.is_empty() {
        return;
    }

    let mut queue = match PENDING_ENTITIES.lock() {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!("Entity queue lock poisoned: {e}");
            return;
        }
    };

    let mut batches_to_flush: Vec<Vec<GraphEntity>> = Vec::new();
    for entity in entities {
        if let Some(batch) = queue.push(entity) {
            batches_to_flush.push(batch);
        }
    }

    // Drop the lock before doing I/O
    drop(queue);

    // Flush all accumulated batches (not just the last one)
    for batch in batches_to_flush {
        let jsonl = graph::merge::entities_to_jsonl(&batch);
        let count = batch.len();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async move {
                match graph::graph_db_load_jsonl(&jsonl).await {
                    Ok(()) => tracing::info!(count, "auto-flushed entity batch to graph"),
                    Err(e) => tracing::warn!("failed to auto-flush entity batch: {e}"),
                }
            })
        });
    }
}

/// Flush and return the pending entities for the index action.
///
/// Returns the entities that were queued but not yet flushed.
pub fn take_pending_entities() -> Vec<GraphEntity> {
    let mut queue = match PENDING_ENTITIES.lock() {
        Ok(q) => q,
        Err(e) => {
            tracing::warn!("Entity queue lock poisoned on take: {e}");
            return Vec::new();
        }
    };
    queue.flush()
}
