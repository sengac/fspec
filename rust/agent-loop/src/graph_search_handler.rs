//! GraphSearch Handler — Concrete Implementation
//!
//! Factory function that creates the per-session handler closure.
//! Routes AST actions to ast_dispatch and Learnings actions to learnings_dispatch.
//!
//! KGRAPH-073: every AST action uses the code graph of one canonical root:
//! the action's `root` when given (a relative root joins the session's
//! working directory), otherwise the session's working directory — the
//! worktree of an isolated session, the session's project otherwise. In an
//! isolated session a root inside the main checkout is refused by the file
//! tools' isolation check. The process working directory never takes part,
//! and every AST result names the root it used.

use codelet_sessions::background_session::BackgroundSession;
use codelet_tools::facade::{validate_and_resolve_path_with_isolation, IsolationContext};
use codelet_tools::graph_search::{GraphSearchAction, GraphSearchHandler};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Weak};
use uuid::Uuid;

use codelet_graph as graph;
use codelet_graph::dispatch_helpers::with_root;
use codelet_graph::registry::GraphRead;

/// Where a GraphSearch call's paths resolve: the session's working
/// directory and, for an isolated session, its isolation context.
pub struct SessionRoot {
    pub working_directory: PathBuf,
    pub isolation: Option<IsolationContext>,
}

/// Produces the [`SessionRoot`] at the time of each call, so a session that
/// enters or leaves isolation moves its graph with it.
pub type RootSource = Arc<dyn Fn() -> Result<SessionRoot, String> + Send + Sync>;

/// The root source of a session: its worktree when isolated, else its project.
pub fn session_root_source(session: &Arc<BackgroundSession>) -> RootSource {
    let session: Weak<BackgroundSession> = Arc::downgrade(session);
    Arc::new(move || {
        let session = session
            .upgrade()
            .ok_or_else(|| "The session of this GraphSearch call has ended".to_string())?;
        let project = PathBuf::from(&session.project);
        let isolation = session
            .worktree_path()
            .map(|worktree_path| IsolationContext {
                worktree_path,
                blocked_project_path: project,
            });
        Ok(SessionRoot {
            working_directory: session.effective_cwd(),
            isolation,
        })
    })
}

/// A root source fixed to one directory (DeepSearch sub-agents).
pub fn fixed_root_source(directory: PathBuf) -> RootSource {
    Arc::new(move || {
        Ok(SessionRoot {
            working_directory: directory.clone(),
            isolation: None,
        })
    })
}

/// Register the production GraphSearch handler for a session.
pub fn register_for_session(session: &Arc<BackgroundSession>) {
    codelet_tools::set_graph_search_handler(
        session.id,
        Some(create_handler(session_root_source(session))),
    );
}

/// Create a GraphSearch handler whose paths resolve through `roots`.
///
/// The returned handler closure dispatches to the appropriate graph:
/// - AST actions → ast_dispatch (via the code graph of the resolved root)
/// - Learnings actions → learnings_dispatch (via the learnings graph)
pub fn create_handler(roots: RootSource) -> GraphSearchHandler {
    Arc::new(move |action: GraphSearchAction, _session_id: Uuid| {
        let roots = roots.clone();
        // Use tokio::task::block_in_place to run async code from sync context
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(async move { dispatch_action(&roots, action).await })
        })
    })
}

fn error_json(action_name: &str, error: impl std::fmt::Display) -> String {
    serde_json::json!({ "action": action_name, "error": error.to_string() }).to_string()
}

/// Resolve a path a GraphSearch call names (a root or a bundle file).
fn resolve_session_path(roots: &RootSource, requested: Option<&str>) -> Result<PathBuf, String> {
    let SessionRoot {
        working_directory,
        isolation,
    } = roots()?;
    let Some(requested) = requested else {
        return Ok(working_directory);
    };
    match isolation {
        Some(isolation) => {
            validate_and_resolve_path_with_isolation(requested, Some(&isolation), "graph_search")
                .map_err(|error| error.to_string())
        }
        None => {
            let requested = Path::new(requested);
            Ok(if requested.is_absolute() {
                requested.to_path_buf()
            } else {
                working_directory.join(requested)
            })
        }
    }
}

/// The canonical root of an AST action.
fn resolve_root(roots: &RootSource, requested: Option<&str>) -> Result<PathBuf, String> {
    graph::registry::canonical_root(&resolve_session_path(roots, requested)?)
}

/// Run `query` against the code graph of the action's root and name the
/// root in the result.
async fn with_code_graph<F, Fut>(
    roots: &RootSource,
    requested: Option<&str>,
    action_name: &str,
    query: F,
) -> String
where
    F: FnOnce(GraphRead) -> Fut,
    Fut: std::future::Future<Output = String>,
{
    let root = match resolve_root(roots, requested) {
        Ok(root) => root,
        Err(error) => return error_json(action_name, error),
    };
    let output = match graph::registry::read_code_graph(&root).await {
        Ok(graph) => query(graph).await,
        Err(error) => error_json(action_name, error),
    };
    with_root(output, &root)
}

/// Get the learnings graph or return a formatted JSON error string.
async fn learnings_or_err(action_name: &str) -> Result<GraphRead, String> {
    graph::registry::read_learnings_graph()
        .await
        .map_err(|e| error_json(action_name, e))
}

/// Dispatch a GraphSearch action to the appropriate graph dispatch function.
async fn dispatch_action(roots: &RootSource, action: GraphSearchAction) -> String {
    match action {
        // ── AST Graph Actions ──────────────────────────────────
        GraphSearchAction::AstSearch {
            root,
            query,
            entity_type,
            limit,
            path,
            search_mode,
            decorator,
            parameter,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_search", |db| async move {
                graph::ast_dispatch::dispatch_ast_search(
                    &db,
                    &query,
                    entity_type.as_deref(),
                    limit,
                    path.as_deref(),
                    search_mode.as_deref(),
                    decorator.as_deref(),
                    parameter.as_deref(),
                )
                .await
            })
            .await
        }
        GraphSearchAction::AstNeighbors {
            root,
            node_id,
            depth,
            edge_types,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_neighbors", |db| async move {
                graph::ast_dispatch::dispatch_ast_neighbors(
                    &db,
                    &node_id,
                    depth,
                    edge_types.as_deref(),
                )
                .await
            })
            .await
        }
        GraphSearchAction::AstStats { root } => {
            with_code_graph(roots, root.as_deref(), "ast_stats", |db| async move {
                graph::ast_dispatch::dispatch_ast_stats(&db).await
            })
            .await
        }

        GraphSearchAction::AstIndex {
            root,
            path,
            reset,
            incremental,
        } => {
            if let Some(path) = path {
                return error_json(
                    "ast_index",
                    format!(
                        "ast_index takes the directory to index as root, not path \
                         (path is the glob filter of queries); got path '{path}'"
                    ),
                );
            }
            match resolve_root(roots, root.as_deref()) {
                Ok(root) => {
                    graph::ast_index::dispatch_ast_index(
                        &root,
                        reset.unwrap_or(false),
                        incremental.unwrap_or(false),
                    )
                    .await
                }
                Err(error) => error_json("ast_index", error),
            }
        }

        GraphSearchAction::AstDeadCode {
            root,
            entity_type,
            limit,
            path,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_dead_code", |db| async move {
                graph::ast_dead_code::dispatch_ast_dead_code(
                    &db,
                    entity_type.as_deref(),
                    limit,
                    path.as_deref(),
                )
                .await
            })
            .await
        }

        GraphSearchAction::AstCallChain {
            root,
            from,
            to,
            max_depth,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_call_chain", |db| async move {
                graph::ast_call_chain::dispatch_ast_call_chain(&db, &from, &to, max_depth).await
            })
            .await
        }

        GraphSearchAction::AstCallers {
            root,
            node_id,
            max_depth,
            limit,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_callers", |db| async move {
                graph::ast_transitive::dispatch_ast_callers(&db, &node_id, max_depth, limit).await
            })
            .await
        }

        GraphSearchAction::AstCallees {
            root,
            node_id,
            max_depth,
            limit,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_callees", |db| async move {
                graph::ast_transitive::dispatch_ast_callees(&db, &node_id, max_depth, limit).await
            })
            .await
        }

        GraphSearchAction::AstHierarchy {
            root,
            node_id,
            depth,
            include_methods,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_hierarchy", |db| async move {
                graph::ast_hierarchy::dispatch_ast_hierarchy(&db, &node_id, depth, include_methods)
                    .await
            })
            .await
        }

        GraphSearchAction::AstComplexity {
            root,
            node_id,
            limit,
            min_threshold,
            path,
        } => {
            with_code_graph(roots, root.as_deref(), "ast_complexity", |db| async move {
                graph::ast_complexity::dispatch_ast_complexity(
                    &db,
                    node_id.as_deref(),
                    limit,
                    min_threshold,
                    path.as_deref(),
                )
                .await
            })
            .await
        }

        GraphSearchAction::AstExport { root, output_path } => {
            let output = match resolve_session_path(roots, Some(&output_path)) {
                Ok(output) => output,
                Err(error) => return error_json("ast_export", error),
            };
            with_code_graph(roots, root.as_deref(), "ast_export", |db| async move {
                match db
                    .export_bundle(&output, graph::registry::AST_CODE_SCHEMA)
                    .await
                {
                    Ok(()) => {
                        let stats = db.stats().unwrap_or_default();
                        // KGRAPH-077: name the graph version exported.
                        graph::dispatch_helpers::stamp(
                            &db,
                            serde_json::json!({
                                "action": "ast_export",
                                "output_path": output.display().to_string(),
                                "stats": stats,
                            }),
                        )
                    }
                    Err(e) => error_json("ast_export", e),
                }
            })
            .await
        }

        GraphSearchAction::AstImport {
            root,
            input_path,
            merge_mode,
        } => {
            let (root, input) = match resolve_root(roots, root.as_deref())
                .and_then(|root| Ok((root, resolve_session_path(roots, Some(&input_path))?)))
            {
                Ok(resolved) => resolved,
                Err(error) => return error_json("ast_import", error),
            };
            let output = import_bundle(&root, &input, merge_mode.as_deref()).await;
            with_root(output, &root)
        }

        // ── Learnings Graph Actions ──────────────────────────────
        GraphSearchAction::LearningsSearch {
            query,
            category,
            limit,
        } => {
            let db = match learnings_or_err("learnings_search").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_search(
                &db,
                &query,
                category.as_deref(),
                limit,
            )
            .await
        }
        GraphSearchAction::LearningsDecisions { domain, status } => {
            let db = match learnings_or_err("learnings_decisions").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_decisions(
                &db,
                domain.as_deref(),
                status.as_deref(),
            )
            .await
        }
        GraphSearchAction::LearningsStats => {
            let db = match learnings_or_err("learnings_stats").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_stats(&db).await
        }
        GraphSearchAction::LearningsRelated {
            topic,
            min_strength,
            limit,
        } => {
            let db = match learnings_or_err("learnings_related").await {
                Ok(db) => db,
                Err(err_json) => return err_json,
            };
            graph::learnings_dispatch::dispatch_learnings_related(&db, &topic, min_strength, limit)
                .await
        }
    }
}

/// Import a bundle into the code graph of `root` under its write lease.
async fn import_bundle(root: &Path, input: &Path, merge_mode: Option<&str>) -> String {
    let mode = merge_mode.unwrap_or("overwrite");
    let mut lease = match graph::registry::write_code_graph(root).await {
        Ok(lease) => lease,
        Err(e) => return error_json("ast_import", e),
    };
    let db = match lease.database().await {
        Ok(db) => db,
        Err(e) => return error_json("ast_import", e),
    };
    match db
        .import_bundle(input, graph::registry::AST_CODE_SCHEMA, mode)
        .await
    {
        Ok(()) => {
            let stats = db.stats().unwrap_or_default();
            // KGRAPH-077: name the graph version the import wrote.
            graph::dispatch_helpers::stamp(
                db,
                serde_json::json!({
                    "action": "ast_import",
                    "input_path": input.display().to_string(),
                    "mode": mode,
                    "stats": stats,
                }),
            )
        }
        Err(e) => error_json("ast_import", e),
    }
}
