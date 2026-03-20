//! GraphSearch Handler — Concrete Implementation
//!
//! Factory function that creates the per-session handler closure.
//! The closure delegates to `graph::dispatch::*` functions.
//!
//! Registered by session_manager.rs at session start.

use codelet_tools::graph_search::{GraphSearchAction, GraphSearchHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::graph;

/// Create a GraphSearch handler for a session.
///
/// The returned handler closure:
/// 1. Ensures the graph DB is initialized (auto-inits on first use)
/// 2. Dispatches to the appropriate graph dispatch function
/// 3. Returns JSON string results
///
/// KGRAPH-012: Captures provider_name and model_id from the session context
/// so that dispatch_index can use them for LLM extraction.
pub fn create_handler() -> GraphSearchHandler {
    create_handler_with_provider(None, None)
}

/// Create a GraphSearch handler with provider context for LLM extraction.
///
/// When provider_name is Some, the index action with scope="all" will run
/// LLM-based concept extraction in addition to structural extraction.
pub fn create_handler_with_provider(
    provider_name: Option<String>,
    model_id: Option<String>,
) -> GraphSearchHandler {
    Arc::new(move |action: GraphSearchAction, _session_id: Uuid| {
        let prov = provider_name.clone();
        let model = model_id.clone();
        // Use tokio::task::block_in_place to run async code from sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                dispatch_action(action, prov.as_deref(), model.as_deref()).await
            })
        })
    })
}

/// Dispatch a GraphSearch action to the appropriate graph dispatch function.
async fn dispatch_action(
    action: GraphSearchAction,
    provider_name: Option<&str>,
    model_id: Option<&str>,
) -> String {
    // Ensure DB is initialized for all actions
    if let Err(e) = graph::ensure_graph_db().await {
        return format!(r#"{{"error":"{}"}}"#, e);
    }

    match action {
        GraphSearchAction::Stats => match graph::graph_db_stats().await {
            Ok(stats) => stats,
            Err(e) => format!(r#"{{"error":"{}"}}"#, e),
        },

        GraphSearchAction::Search { query, category, limit } => {
            graph::dispatch::dispatch_search(&query, category.as_deref(), limit.map(|l| l as u32)).await
        }

        GraphSearchAction::Neighbors { node_id, depth, edge_types } => {
            graph::dispatch::dispatch_neighbors(&node_id, depth.map(|d| d as u32), edge_types).await
        }

        GraphSearchAction::Path { from, to, max_hops } => {
            // Path queries require multi-hop traversal — return placeholder with concept info
            let max = max_hops.unwrap_or(5);
            format!(
                r#"{{"action":"path","from":"{}","to":"{}","max_hops":{},"paths":[],"note":"Multi-hop path queries are not yet supported by the graph query engine"}}"#,
                from, to, max
            )
        }

        GraphSearchAction::Related { topic, min_strength, limit } => {
            graph::dispatch::dispatch_related(&topic, min_strength, limit.map(|l| l as u32)).await
        }

        GraphSearchAction::Decisions { domain, status, since } => {
            graph::dispatch::dispatch_decisions(domain.as_deref(), status.as_deref(), since.as_deref()).await
        }

        GraphSearchAction::History { concept, limit } => {
            graph::dispatch::dispatch_history(&concept, limit.map(|l| l as u32)).await
        }

        GraphSearchAction::Index { scope } => {
            graph::dispatch::dispatch_index(scope.as_deref(), provider_name, model_id).await
        }
    }
}

// ── Entity Pipeline Delegation ───────────────────────────────────────────
// Moved to graph::entity_pipeline — re-export public API for backward compatibility.

pub use crate::graph::entity_pipeline::{
    extract_and_queue_from_tool_call,
    flush_pending_entities,
};
