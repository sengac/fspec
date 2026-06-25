//! Learnings Graph Dispatch Functions
//!
//! Implements query dispatch logic for Learnings-specific GraphSearch actions.
//! Routes queries to the Learnings graph database (dual-graph architecture).
//!
//! Each function takes a `GraphDatabase` reference and returns a JSON string.
//! Follows the exact same pattern as `ast_dispatch.rs`.

use crate::database::GraphDatabase;
use crate::dispatch_helpers::{format_graph_stats, matches_fields, LEARNINGS_SEARCHABLE_FIELDS};
use serde_json::Value;
use tracing::warn;

/// Bundled Learnings query source for named queries.
pub const LEARNINGS_QUERIES: &str = include_str!("../schemas/learnings-queries.gq");

/// Search Learnings entities by text/category.
///
/// Searches across Learning, Decision, Convention, and Exploration node types.
/// Supports optional category filter to narrow results.
/// Uses client-side filtering on all nodes of each type.
pub async fn dispatch_learnings_search(
    db: &GraphDatabase,
    query: &str,
    category: Option<&str>,
    limit: Option<usize>,
) -> String {
    let query_lower = query.to_lowercase();
    let max_results = limit.unwrap_or(20);
    let mut results = Vec::new();

    // Search across all learnings node types
    let search_types = [
        "Learning",
        "Decision",
        "Convention",
        "Exploration",
        "CodePattern",
    ];

    for node_type in search_types {
        let query_name = match node_type {
            "Learning" => "all_learnings",
            "Decision" => "all_decisions",
            "Convention" => "all_conventions",
            "Exploration" => "all_explorations",
            "CodePattern" => "all_code_patterns",
            _ => continue,
        };

        match db
            .query_with_source(LEARNINGS_QUERIES, query_name, None)
            .await
        {
            Ok(Value::Array(items)) => {
                for item in items {
                    if results.len() >= max_results {
                        break;
                    }

                    // Apply category filter if specified
                    if let Some(cat) = category {
                        if let Some(item_cat) = item.get("category").and_then(|v| v.as_str()) {
                            if item_cat != cat {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    if matches_fields(&item, &query_lower, LEARNINGS_SEARCHABLE_FIELDS) {
                        results.push(item);
                    }
                }
            }
            Ok(_) => { /* query returned non-array result; skip */ }
            Err(e) => {
                warn!(query_name, error = %e, "Learnings search query failed");
            }
        }
    }

    let count = results.len();
    serde_json::json!({
        "action": "learnings_search",
        "query": query,
        "category": category,
        "results": results,
        "count": count,
    })
    .to_string()
}

/// Query Decision nodes filtered by domain and/or status.
///
/// Returns decisions matching the optional domain and status filters.
/// Both filters are applied conjunctively (AND logic).
pub async fn dispatch_learnings_decisions(
    db: &GraphDatabase,
    domain: Option<&str>,
    status: Option<&str>,
) -> String {
    let mut results = Vec::new();

    match db
        .query_with_source(LEARNINGS_QUERIES, "all_decisions", None)
        .await
    {
        Ok(Value::Array(items)) => {
            for item in items {
                // Apply domain filter
                if let Some(d) = domain {
                    if item.get("domain").and_then(|v| v.as_str()) != Some(d) {
                        continue;
                    }
                }

                // Apply status filter
                if let Some(s) = status {
                    if item.get("status").and_then(|v| v.as_str()) != Some(s) {
                        continue;
                    }
                }

                results.push(item);
            }
        }
        Ok(_) => { /* query returned non-array result; skip */ }
        Err(e) => {
            warn!(error = %e, "Learnings decisions query failed");
        }
    }

    let count = results.len();
    serde_json::json!({
        "action": "learnings_decisions",
        "domain": domain,
        "status": status,
        "results": results,
        "count": count,
    })
    .to_string()
}

/// Get Learnings graph statistics.
///
/// Uses the GraphDatabase's built-in `stats()` method for accurate counts
/// directly from storage segments. Same pattern as `dispatch_ast_stats`.
pub async fn dispatch_learnings_stats(db: &GraphDatabase) -> String {
    format_graph_stats(db, "learnings_stats")
}

/// Find learnings related to a topic.
///
/// First finds Learning nodes matching the topic via text search,
/// then follows RelatesTo edges to find connected learnings.
/// Results include the relationship strength and type.
pub async fn dispatch_learnings_related(
    db: &GraphDatabase,
    topic: &str,
    min_strength: Option<f32>,
    limit: Option<usize>,
) -> String {
    let topic_lower = topic.to_lowercase();
    let max_results = limit.unwrap_or(20);
    let min_str = min_strength.unwrap_or(0.0);
    let mut results = Vec::new();

    // First find Learning nodes matching the topic
    match db
        .query_with_source(LEARNINGS_QUERIES, "all_learnings", None)
        .await
    {
        Ok(Value::Array(items)) => {
            for item in items {
                if results.len() >= max_results {
                    break;
                }
                if matches_fields(&item, &topic_lower, LEARNINGS_SEARCHABLE_FIELDS) {
                    results.push(item);
                }
            }
        }
        Ok(_) => { /* query returned non-array result; skip */ }
        Err(e) => {
            warn!(error = %e, "Learnings related query failed");
        }
    }

    // For each matched learning, find related nodes via RelatesTo edges
    let mut related = Vec::new();
    for result in &results {
        if let Some(slug) = result.get("slug").and_then(|v| v.as_str()) {
            let params = serde_json::json!({ "slug": slug });
            if let Ok(Value::Array(neighbors)) = db
                .query_with_source(LEARNINGS_QUERIES, "learning_related", Some(&params))
                .await
            {
                for neighbor in neighbors {
                    // Apply min_strength filter if the neighbor has a strength value
                    if min_str > 0.0 {
                        if let Some(strength) = neighbor.get("strength").and_then(|v| v.as_f64()) {
                            if (strength as f32) < min_str {
                                continue;
                            }
                        }
                    }
                    if related.len() < max_results {
                        related.push(neighbor);
                    }
                }
            }
        }
    }

    // Combine direct matches and related
    let all_results: Vec<_> = results.into_iter().chain(related).collect();
    let count = all_results.len();

    serde_json::json!({
        "action": "learnings_related",
        "topic": topic,
        "results": all_results,
        "count": count,
    })
    .to_string()
}
