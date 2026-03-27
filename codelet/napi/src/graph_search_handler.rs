//! GraphSearch Handler — Concrete Implementation
//!
//! Factory function that creates the per-session handler closure.
//! Routes AST actions to ast_dispatch and Learnings actions to learnings_dispatch.
//!
//! Registered by session_manager.rs at session start.

use codelet_tools::graph_search::{GraphSearchAction, GraphSearchHandler};
use std::sync::Arc;
use uuid::Uuid;

use crate::graph;
use crate::graph::database::GraphDatabase;

/// Create a GraphSearch handler for a session.
///
/// The returned handler closure dispatches to the appropriate graph:
/// - AST actions → ast_dispatch (via ast-code graph)
/// - Learnings actions → learnings_dispatch (via learnings graph)
pub fn create_handler() -> GraphSearchHandler {
    Arc::new(move |action: GraphSearchAction, _session_id: Uuid| {
        // Use tokio::task::block_in_place to run async code from sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                dispatch_action(action).await
            })
        })
    })
}

/// Get a named graph or return a formatted JSON error string.
///
/// Eliminates the repetitive `match get_graph(...) { Ok/Err }` boilerplate
/// across all action dispatch arms.
async fn get_graph_or_err(
    name: &str,
    action_name: &str,
) -> Result<GraphDatabase, String> {
    graph::registry::get_graph(name).await.map_err(|e| {
        serde_json::json!({
            "action": action_name,
            "error": e,
        })
        .to_string()
    })
}

/// Dispatch a GraphSearch action to the appropriate graph dispatch function.
async fn dispatch_action(action: GraphSearchAction) -> String {
    match action {
        // ── AST Graph Actions ──────────────────────────────────
        GraphSearchAction::AstSearch { query, entity_type, limit, path, search_mode, decorator, parameter } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_search").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_dispatch::dispatch_ast_search(
                &db, &query, entity_type.as_deref(), limit, path.as_deref(),
                search_mode.as_deref(), decorator.as_deref(), parameter.as_deref(),
            ).await
        }
        GraphSearchAction::AstNeighbors { node_id, depth, edge_types } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_neighbors").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_dispatch::dispatch_ast_neighbors(
                &db, &node_id, depth, edge_types.as_deref(),
            ).await
        }
        GraphSearchAction::AstStats => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_stats").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_dispatch::dispatch_ast_stats(&db).await
        }

        GraphSearchAction::AstIndex { path, reset, incremental } => {
            graph::ast_index::dispatch_ast_index(
                path.as_deref(),
                reset.unwrap_or(false),
                incremental.unwrap_or(false),
            ).await
        }

        GraphSearchAction::AstDeadCode { entity_type, limit, path } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_dead_code").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_dead_code::dispatch_ast_dead_code(
                &db, entity_type.as_deref(), limit, path.as_deref(),
            ).await
        }

        GraphSearchAction::AstCallChain { from, to, max_depth } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_call_chain").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_call_chain::dispatch_ast_call_chain(
                &db, &from, &to, max_depth,
            ).await
        }

        GraphSearchAction::AstCallers { node_id, max_depth, limit } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_callers").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_transitive::dispatch_ast_callers(
                &db, &node_id, max_depth, limit,
            ).await
        }

        GraphSearchAction::AstCallees { node_id, max_depth, limit } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_callees").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_transitive::dispatch_ast_callees(
                &db, &node_id, max_depth, limit,
            ).await
        }

        GraphSearchAction::AstHierarchy { node_id, depth, include_methods } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_hierarchy").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_hierarchy::dispatch_ast_hierarchy(
                &db, &node_id, depth, include_methods,
            ).await
        }

        GraphSearchAction::AstComplexity { node_id, limit, min_threshold, path } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_complexity").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::ast_complexity::dispatch_ast_complexity(
                &db,
                node_id.as_deref(),
                limit,
                min_threshold,
                path.as_deref(),
            ).await
        }

        GraphSearchAction::AstExport { output_path } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_export").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            let schema = include_str!("graph/../../schemas/ast-code.pg");
            let path = std::path::Path::new(&output_path);
            match db.export_bundle(path, schema).await {
                Ok(()) => {
                    let stats = db.stats().unwrap_or_default();
                    serde_json::json!({
                        "action": "ast_export",
                        "output_path": output_path,
                        "stats": stats,
                    }).to_string()
                }
                Err(e) => serde_json::json!({ "error": e }).to_string(),
            }
        }

        GraphSearchAction::AstImport { input_path, merge_mode } => {
            let db = match get_graph_or_err(graph::registry::AST_CODE_GRAPH, "ast_import").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            let schema = include_str!("graph/../../schemas/ast-code.pg");
            let path = std::path::Path::new(&input_path);
            let mode = merge_mode.as_deref().unwrap_or("overwrite");
            match db.import_bundle(path, schema, mode).await {
                Ok(()) => {
                    let stats = db.stats().unwrap_or_default();
                    serde_json::json!({
                        "action": "ast_import",
                        "input_path": input_path,
                        "mode": mode,
                        "stats": stats,
                    }).to_string()
                }
                Err(e) => serde_json::json!({ "error": e }).to_string(),
            }
        }

        // ── Learnings Graph Actions ──────────────────────────────
        GraphSearchAction::LearningsSearch { query, category, limit } => {
            let db = match get_graph_or_err(graph::registry::LEARNINGS_GRAPH, "learnings_search").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_search(
                &db, &query, category.as_deref(), limit,
            ).await
        }
        GraphSearchAction::LearningsDecisions { domain, status } => {
            let db = match get_graph_or_err(graph::registry::LEARNINGS_GRAPH, "learnings_decisions").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_decisions(
                &db, domain.as_deref(), status.as_deref(),
            ).await
        }
        GraphSearchAction::LearningsStats => {
            let db = match get_graph_or_err(graph::registry::LEARNINGS_GRAPH, "learnings_stats").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_stats(&db).await
        }
        GraphSearchAction::LearningsRelated { topic, min_strength, limit } => {
            let db = match get_graph_or_err(graph::registry::LEARNINGS_GRAPH, "learnings_related").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_related(
                &db, &topic, min_strength, limit,
            ).await
        }
    }
}
