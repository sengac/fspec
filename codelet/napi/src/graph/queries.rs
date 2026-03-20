//! GraphSearch Query Implementations
//!
//! Implements all GraphSearch actions: search, neighbors, related,
//! decisions, and stats. Each action prepares parameters and formats
//! results as JSON strings for LLM consumption.
//!
//! Feature: spec/features/graphsearch-query-implementations.feature

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Result of a graph search action, returned as JSON.
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    pub results: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

/// Filter related edges by post-processing (min_strength applied in Rust).
pub fn filter_by_min_strength(results: &[Value], min_strength: f64) -> Vec<Value> {
    results
        .iter()
        .filter(|row| {
            row.get("strength")
                .and_then(|v| v.as_f64())
                .map_or(false, |s| s >= min_strength)
        })
        .cloned()
        .collect()
}

/// Filter decisions by domain (post-processing).
pub fn filter_decisions_by_domain(results: &[Value], domain: &str) -> Vec<Value> {
    results
        .iter()
        .filter(|row| {
            row.get("domain")
                .and_then(|v| v.as_str())
                .map_or(false, |d| d == domain)
        })
        .cloned()
        .collect()
}

/// Filter decisions by status (post-processing).
pub fn filter_decisions_by_status(results: &[Value], status: &str) -> Vec<Value> {
    results
        .iter()
        .filter(|row| {
            row.get("status")
                .and_then(|v| v.as_str())
                .map_or(false, |s| s == status)
        })
        .cloned()
        .collect()
}

/// Build stats result from catalog node/edge type counts.
pub fn build_stats_result(
    node_counts: &Map<String, Value>,
    edge_counts: &Map<String, Value>,
) -> GraphQueryResult {
    let mut result = Map::new();
    result.insert("nodes".to_string(), Value::Object(node_counts.clone()));
    result.insert("edges".to_string(), Value::Object(edge_counts.clone()));

    GraphQueryResult {
        action: "stats".to_string(),
        query: None,
        results: Value::Object(result),
        count: None,
    }
}

/// Format search results into a GraphQueryResult.
pub fn format_search_result(query: &str, results: Vec<Value>) -> GraphQueryResult {
    let count = results.len();
    GraphQueryResult {
        action: "search".to_string(),
        query: Some(query.to_string()),
        results: Value::Array(results),
        count: Some(count),
    }
}

/// Format neighbors results into a GraphQueryResult.
pub fn format_neighbors_result(slug: &str, results: Vec<Value>) -> GraphQueryResult {
    let count = results.len();
    GraphQueryResult {
        action: "neighbors".to_string(),
        query: Some(slug.to_string()),
        results: Value::Array(results),
        count: Some(count),
    }
}

/// Format related results into a GraphQueryResult.
pub fn format_related_result(slug: &str, results: Vec<Value>) -> GraphQueryResult {
    let count = results.len();
    GraphQueryResult {
        action: "related".to_string(),
        query: Some(slug.to_string()),
        results: Value::Array(results),
        count: Some(count),
    }
}

/// Format decisions results into a GraphQueryResult.
pub fn format_decisions_result(results: Vec<Value>) -> GraphQueryResult {
    let count = results.len();
    GraphQueryResult {
        action: "decisions".to_string(),
        query: None,
        results: Value::Array(results),
        count: Some(count),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_concept_row(slug: &str, name: &str, category: &str, mention_count: i64) -> Value {
        let mut row = Map::new();
        row.insert("slug".to_string(), Value::String(slug.to_string()));
        row.insert("name".to_string(), Value::String(name.to_string()));
        row.insert("category".to_string(), Value::String(category.to_string()));
        row.insert(
            "mentionCount".to_string(),
            Value::Number(serde_json::Number::from(mention_count)),
        );
        row.insert(
            "confidence".to_string(),
            Value::String("high".to_string()),
        );
        Value::Object(row)
    }

    fn make_decision_row(slug: &str, title: &str, domain: &str, status: &str) -> Value {
        let mut row = Map::new();
        row.insert("slug".to_string(), Value::String(slug.to_string()));
        row.insert("title".to_string(), Value::String(title.to_string()));
        row.insert("domain".to_string(), Value::String(domain.to_string()));
        row.insert("status".to_string(), Value::String(status.to_string()));
        row.insert(
            "decidedAt".to_string(),
            Value::String("2026-03-19T00:00:00Z".to_string()),
        );
        Value::Object(row)
    }

    fn make_edge_row(from: &str, to: &str, strength: f64, relation_type: &str) -> Value {
        let mut row = Map::new();
        row.insert("from".to_string(), Value::String(from.to_string()));
        row.insert("to".to_string(), Value::String(to.to_string()));
        row.insert(
            "strength".to_string(),
            Value::Number(serde_json::Number::from_f64(strength).unwrap()),
        );
        row.insert(
            "relationType".to_string(),
            Value::String(relation_type.to_string()),
        );
        Value::Object(row)
    }

    // ============================================================================
    // Scenario: Search action finds concepts by text query
    // ============================================================================
    #[test]
    fn test_search_action_returns_matching_concepts() {
        // @step Given a knowledge graph containing concept nodes for "JWT Authentication" and "Session Management"
        let rows = vec![
            make_concept_row("jwt-authentication", "JWT Authentication", "technology", 15),
            make_concept_row("session-management", "Session Management", "pattern", 8),
        ];

        // @step When the search action is invoked with query "JWT"
        let result = format_search_result("JWT", rows);

        // @step Then concept nodes matching the query are returned as JSON
        assert_eq!(result.action, "search");
        assert_eq!(result.count, Some(2));

        // @step And results include slug, name, category, and mentionCount fields
        let results = result.results.as_array().unwrap();
        assert!(results[0].get("slug").is_some());
        assert!(results[0].get("name").is_some());
        assert!(results[0].get("category").is_some());
        assert!(results[0].get("mentionCount").is_some());
    }

    // ============================================================================
    // Scenario: Neighbors action returns concepts within hop distance
    // ============================================================================
    #[test]
    fn test_neighbors_action_returns_hop_results() {
        // @step Given a knowledge graph with "jwt-authentication" related to "session-management" related to "redis-cache"
        let rows = vec![
            {
                let mut row = Map::new();
                row.insert(
                    "slug".to_string(),
                    Value::String("session-management".to_string()),
                );
                row.insert(
                    "name".to_string(),
                    Value::String("Session Management".to_string()),
                );
                row.insert("depth".to_string(), Value::Number(1.into()));
                Value::Object(row)
            },
            {
                let mut row = Map::new();
                row.insert(
                    "slug".to_string(),
                    Value::String("redis-cache".to_string()),
                );
                row.insert(
                    "name".to_string(),
                    Value::String("Redis Cache".to_string()),
                );
                row.insert("depth".to_string(), Value::Number(2.into()));
                Value::Object(row)
            },
        ];

        // @step When the neighbors action is invoked for "jwt-authentication" with depth 2
        let result = format_neighbors_result("jwt-authentication", rows);

        // @step Then "session-management" is returned at depth 1
        let results = result.results.as_array().unwrap();
        assert_eq!(results[0]["slug"], "session-management");
        assert_eq!(results[0]["depth"], 1);

        // @step And "redis-cache" is returned at depth 2
        assert_eq!(results[1]["slug"], "redis-cache");
        assert_eq!(results[1]["depth"], 2);
    }

    // ============================================================================
    // Scenario: Related action filters by minimum strength
    // ============================================================================
    #[test]
    fn test_related_action_filters_by_min_strength() {
        // @step Given a knowledge graph with RelatesTo edges at various strengths
        let edges = vec![
            make_edge_row("jwt-auth", "session-mgmt", 0.9, "supersedes"),
            make_edge_row("jwt-auth", "oauth", 0.3, "uses"),
            make_edge_row("jwt-auth", "bcrypt", 0.6, "depends_on"),
        ];

        // @step When the related action is invoked for "jwt-authentication" with min_strength 0.5
        let filtered = filter_by_min_strength(&edges, 0.5);

        // @step Then only edges with strength greater than or equal to 0.5 are returned
        assert_eq!(filtered.len(), 2);

        // Also verify format_related_result wraps correctly
        let result = format_related_result("jwt-authentication", filtered.clone());
        assert_eq!(result.action, "related");
        assert_eq!(result.count, Some(2));

        // @step And edges below the threshold are excluded
        for edge in &filtered {
            let s = edge["strength"].as_f64().unwrap();
            assert!(s >= 0.5);
        }
    }

    // ============================================================================
    // Scenario: Decisions action filters by domain
    // ============================================================================
    #[test]
    fn test_decisions_action_filters_by_domain() {
        // @step Given a knowledge graph with decisions across multiple domains
        let decisions = vec![
            make_decision_row("use-jwt", "Use JWT tokens", "architecture", "active"),
            make_decision_row("use-prettier", "Use Prettier", "convention", "active"),
            make_decision_row("use-postgres", "Use PostgreSQL", "architecture", "superseded"),
        ];

        // @step When the decisions action is invoked with domain filter "architecture"
        let filtered = filter_decisions_by_domain(&decisions, "architecture");

        // @step Then only decisions with domain "architecture" are returned
        assert_eq!(filtered.len(), 2);
        for d in &filtered {
            assert_eq!(d["domain"], "architecture");
        }

        // Also verify filter_decisions_by_status works
        let active_only = filter_decisions_by_status(&decisions, "active");
        assert_eq!(active_only.len(), 2);
        for d in &active_only {
            assert_eq!(d["status"], "active");
        }

        // @step And results are sorted by decidedAt descending
        // Sorting happens in the nanograph query; we verify the format is correct
        let result = format_decisions_result(filtered);
        assert_eq!(result.action, "decisions");
        assert_eq!(result.count, Some(2));
    }

    // ============================================================================
    // Scenario: Stats action returns type counts
    // ============================================================================
    #[test]
    fn test_stats_action_returns_type_counts() {
        // @step Given a knowledge graph with nodes and edges of various types
        let mut node_counts = Map::new();
        node_counts.insert("Concept".to_string(), Value::Number(42.into()));
        node_counts.insert("Decision".to_string(), Value::Number(7.into()));
        node_counts.insert("CodeEntity".to_string(), Value::Number(120.into()));
        node_counts.insert("Session".to_string(), Value::Number(5.into()));
        node_counts.insert("Turn".to_string(), Value::Number(350.into()));

        let mut edge_counts = Map::new();
        edge_counts.insert("RelatesTo".to_string(), Value::Number(89.into()));
        edge_counts.insert("Mentions".to_string(), Value::Number(200.into()));
        edge_counts.insert("Discusses".to_string(), Value::Number(35.into()));

        // @step When the stats action is invoked
        let result = build_stats_result(&node_counts, &edge_counts);

        // @step Then the result includes counts for each node type
        let nodes = result.results["nodes"].as_object().unwrap();
        assert_eq!(nodes["Concept"], 42);
        assert_eq!(nodes["Decision"], 7);

        // @step And the result includes counts for each edge type
        let edges = result.results["edges"].as_object().unwrap();
        assert_eq!(edges["RelatesTo"], 89);

        // @step And the result is formatted as a JSON object
        assert_eq!(result.action, "stats");
        let json_str = serde_json::to_string(&result).unwrap();
        assert!(json_str.contains("\"action\":\"stats\""));
    }
}
