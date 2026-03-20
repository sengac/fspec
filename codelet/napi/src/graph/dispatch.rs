//! GraphSearch Dispatch Functions
//!
//! Implements the query dispatch logic for each GraphSearch action.
//! Extracted from graph_search_handler.rs to keep files under 300 lines.

use crate::graph;
use serde_json::Value;

/// Bundled graph query source, compiled into the binary.
const GRAPH_QUERIES: &str = include_str!("../../schemas/graph-queries.gq");

/// Search concepts by text query, optionally filtered by category.
pub async fn dispatch_search(query: &str, category: Option<&str>, limit: Option<u32>) -> String {
    let (query_name, params) = if let Some(cat) = category {
        (
            "search_concepts_by_category",
            Some(serde_json::json!({ "query": query, "category": cat })),
        )
    } else if query.is_empty() || query == "*" {
        // Empty/wildcard query → return all concepts
        ("all_concepts", None)
    } else {
        (
            "search_concepts",
            Some(serde_json::json!({ "query": query })),
        )
    };

    match graph::graph_db_query(GRAPH_QUERIES, query_name, params.as_ref()).await {
        Ok(results) => {
            let mut arr = match results {
                Value::Array(a) => a,
                _ => vec![],
            };
            // Apply client-side limit if provided
            if let Some(lim) = limit {
                arr.truncate(lim as usize);
            }
            let count = arr.len();
            serde_json::json!({
                "action": "search",
                "query": query,
                "results": arr,
                "count": count,
            }).to_string()
        }
        Err(e) => {
            // Fallback: if query syntax not supported, return empty results
            tracing::warn!("GraphSearch search query failed: {e}");
            serde_json::json!({
                "action": "search",
                "query": query,
                "results": [],
                "count": 0,
                "note": format!("Query failed: {e}")
            }).to_string()
        }
    }
}

/// Get neighbor concepts via RelatesTo edges.
///
/// Currently supports single-hop traversal via the `concept_neighbors` query.
/// The `depth` parameter controls how many hops (default: 1, max: 3).
/// Multi-hop traversal (depth > 1) requires client-side iterative expansion.
pub async fn dispatch_neighbors(node_id: &str, depth: Option<u32>, edge_types: Option<Vec<String>>) -> String {
    let max_depth = depth.unwrap_or(1).min(3);

    // Collect results across hops
    let mut all_results: Vec<Value> = Vec::new();
    let mut visited_slugs: std::collections::HashSet<String> = std::collections::HashSet::new();
    visited_slugs.insert(node_id.to_string());

    let mut frontier = vec![node_id.to_string()];

    for current_depth in 1..=max_depth {
        let mut next_frontier = Vec::new();

        for slug in &frontier {
            let hop_params = serde_json::json!({ "slug": slug });
            match graph::graph_db_query(GRAPH_QUERIES, "concept_neighbors", Some(&hop_params)).await {
                Ok(results) => {
                    if let Value::Array(rows) = results {
                        for mut row in rows {
                            let neighbor_slug = row.get("slug")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();

                            if visited_slugs.contains(&neighbor_slug) {
                                continue;
                            }
                            visited_slugs.insert(neighbor_slug.clone());

                            // Apply edge_types filter if specified
                            if let Some(ref types) = edge_types {
                                if let Some(rt) = row.get("relationType").and_then(|v| v.as_str()) {
                                    if !types.iter().any(|t| t == rt) {
                                        continue;
                                    }
                                }
                            }

                            // Add depth info to the result
                            if let Value::Object(ref mut map) = row {
                                map.insert(
                                    "depth".to_string(),
                                    Value::Number(current_depth.into()),
                                );
                            }

                            all_results.push(row);
                            next_frontier.push(neighbor_slug);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("GraphSearch neighbors hop {current_depth} query failed for {slug}: {e}");
                }
            }
        }

        frontier = next_frontier;
        if frontier.is_empty() {
            break;
        }
    }

    let count = all_results.len();
    serde_json::json!({
        "action": "neighbors",
        "query": node_id,
        "depth": max_depth,
        "results": all_results,
        "count": count,
    }).to_string()
}

/// Get related concepts with optional strength filtering.
pub async fn dispatch_related(topic: &str, min_strength: Option<f32>, limit: Option<u32>) -> String {
    let params = serde_json::json!({ "slug": topic });
    match graph::graph_db_query(GRAPH_QUERIES, "concept_related", Some(&params)).await {
        Ok(results) => {
            let mut arr = match results {
                Value::Array(a) => a,
                _ => vec![],
            };
            // Client-side min_strength filter
            if let Some(ms) = min_strength {
                arr.retain(|row| {
                    row.get("strength")
                        .and_then(|v| v.as_f64())
                        .map_or(false, |s| s >= ms as f64)
                });
            }
            if let Some(lim) = limit {
                arr.truncate(lim as usize);
            }
            let count = arr.len();
            serde_json::json!({
                "action": "related",
                "query": topic,
                "results": arr,
                "count": count,
            }).to_string()
        }
        Err(e) => {
            tracing::warn!("GraphSearch related query failed: {e}");
            serde_json::json!({
                "action": "related",
                "query": topic,
                "results": [],
                "count": 0,
            }).to_string()
        }
    }
}

/// List decisions with optional domain/status/since filtering.
pub async fn dispatch_decisions(domain: Option<&str>, status: Option<&str>, since: Option<&str>) -> String {
    match graph::graph_db_query(GRAPH_QUERIES, "all_decisions", None).await {
        Ok(results) => {
            let mut arr = match results {
                Value::Array(a) => a,
                _ => vec![],
            };
            // Client-side filtering
            if let Some(d) = domain {
                arr.retain(|row| {
                    row.get("domain")
                        .and_then(|v| v.as_str())
                        .map_or(false, |v| v == d)
                });
            }
            if let Some(s) = status {
                arr.retain(|row| {
                    row.get("status")
                        .and_then(|v| v.as_str())
                        .map_or(false, |v| v == s)
                });
            }
            if let Some(since_ts) = since {
                arr.retain(|row| {
                    row.get("decidedAt")
                        .and_then(|v| v.as_str())
                        .map_or(false, |v| v >= since_ts)
                });
            }
            let count = arr.len();
            serde_json::json!({
                "action": "decisions",
                "results": arr,
                "count": count,
            }).to_string()
        }
        Err(e) => {
            tracing::warn!("GraphSearch decisions query failed: {e}");
            serde_json::json!({
                "action": "decisions",
                "results": [],
                "count": 0,
            }).to_string()
        }
    }
}

/// Get concept history — which turns mentioned this concept.
pub async fn dispatch_history(concept: &str, limit: Option<u32>) -> String {
    let params = serde_json::json!({ "slug": concept });
    match graph::graph_db_query(GRAPH_QUERIES, "concept_history", Some(&params)).await {
        Ok(results) => {
            let mut arr = match results {
                Value::Array(a) => a,
                _ => vec![],
            };
            if let Some(lim) = limit {
                arr.truncate(lim as usize);
            }
            let count = arr.len();
            serde_json::json!({
                "action": "history",
                "concept": concept,
                "results": arr,
                "count": count,
            }).to_string()
        }
        Err(e) => {
            tracing::warn!("GraphSearch history query failed: {e}");
            serde_json::json!({
                "action": "history",
                "concept": concept,
                "results": [],
                "count": 0,
            }).to_string()
        }
    }
}

/// Index action — load structural entities into the graph.
///
/// - scope="current" (default): flushes pending entity queue from real-time tool calls
/// - scope="all": scans all sessions from persistence, extracts structural entities,
///   and (when provider context is available) runs LLM extraction for Concepts/Decisions/Relations
pub async fn dispatch_index(
    scope: Option<&str>,
    provider_name: Option<&str>,
    model_id: Option<&str>,
) -> String {
    let scope_str = scope.unwrap_or("current");

    if scope_str == "all" {
        // Scan all sessions from persistence and index unindexed turns
        // KGRAPH-012: Pass provider context for LLM extraction
        let extraction_mode = if provider_name.is_some() { Some("hybrid") } else { Some("structural") };
        match crate::graph::indexing::scan_and_index_sessions(
            provider_name,
            model_id,
            extraction_mode,
        ).await {
            Ok(result) => {
                // Also flush any pending entities from the current session
                let queue_entities = crate::graph::entity_pipeline::take_pending_entities();
                let mut total_loaded = result.entities_loaded;
                if !queue_entities.is_empty() {
                    let jsonl = graph::merge::entities_to_jsonl(&queue_entities);
                    if let Err(e) = graph::graph_db_load_jsonl(&jsonl).await {
                        tracing::warn!("Failed to flush pending entities during index all: {e}");
                    } else {
                        total_loaded += queue_entities.len() as u32;
                    }
                }

                if total_loaded == 0 && result.sessions_scanned == 0 {
                    return serde_json::json!({
                        "action": "index",
                        "scope": scope_str,
                        "status": "no_unindexed",
                        "sessions_scanned": 0,
                        "sessions_skipped": result.sessions_skipped,
                        "entities_loaded": 0,
                        "message": "All sessions are fully indexed.",
                    }).to_string();
                }

                serde_json::json!({
                    "action": "index",
                    "scope": scope_str,
                    "status": "indexed",
                    "sessions_scanned": result.sessions_scanned,
                    "sessions_skipped": result.sessions_skipped,
                    "entities_loaded": total_loaded,
                }).to_string()
            }
            Err(e) => {
                tracing::error!("GraphSearch index all failed: {e}");
                serde_json::json!({
                    "action": "index",
                    "scope": scope_str,
                    "status": "error",
                    "message": e,
                }).to_string()
            }
        }
    } else {
        // scope="current": flush only the pending entity queue
        let queue_entities = crate::graph::entity_pipeline::take_pending_entities();

        if queue_entities.is_empty() {
            return serde_json::json!({
                "action": "index",
                "scope": scope_str,
                "status": "no_pending",
                "message": "No pending entities to index. Entities are accumulated from Write/Edit/Fspec tool calls.",
            }).to_string();
        }

        let jsonl = graph::merge::entities_to_jsonl(&queue_entities);
        match graph::graph_db_load_jsonl(&jsonl).await {
            Ok(()) => {
                serde_json::json!({
                    "action": "index",
                    "scope": scope_str,
                    "status": "indexed",
                    "entities_loaded": queue_entities.len(),
                }).to_string()
            }
            Err(e) => {
                tracing::error!("GraphSearch index failed: {e}");
                serde_json::json!({
                    "action": "index",
                    "scope": scope_str,
                    "status": "error",
                    "message": e,
                }).to_string()
            }
        }
    }
}
