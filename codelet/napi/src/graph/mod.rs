//! Graph Database Module — Dual-Graph Architecture
//!
//! Provides embedded nanograph property graph databases for the dual-graph
//! architecture:
//!
//! 1. **AST Code Graph** (`"ast-code"`) — Code structure, dependencies, and relationships
//!    stored at `<project>/.fspec/graph/ast-code.nano/`
//! 2. **Learnings Graph** (`"learnings"`) — Accumulated knowledge, decisions, and conventions
//!    stored at `~/.fspec/graph/learnings.nano/`
//!
//! Uses a registry of named graph instances (see `registry.rs`).

/// Close all graph databases cleanly.
///
/// Should be called on process exit to avoid Lance corruption.
pub fn close_graph_db() {
    registry::close_all_graphs();
}

/// Reset all graph databases.
///
/// Called when the data directory changes (via `set_data_directory()`).
pub fn reset_graph_db() {
    registry::reset_all_graphs();
}

/// Populate the AST code graph from the current project directory.
///
/// Walks the codebase extracting functions, types, imports, and dependencies,
/// then batch-loads everything into the AST graph. Silently skips if the
/// graph is unavailable.
///
/// Called at session start so the GraphSearch tool has data to query.
pub async fn populate_ast_graph() {
    let project_root = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to get cwd for AST indexing: {e}");
            return;
        }
    };

    let db = match registry::get_graph(registry::AST_CODE_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to open AST graph for indexing: {e}");
            return;
        }
    };

    // Walk codebase and extract AST entities
    let mut all_entities = match ast_pipeline::walk_and_extract(&project_root) {
        Ok(entities) => entities,
        Err(e) => {
            tracing::warn!("[KGRAPH] AST extraction failed: {e}");
            return;
        }
    };

    // Extract dependencies (non-fatal failures)
    if let Ok(cargo_deps) =
        ast_pipeline::cargo_dep_extractor::extract_cargo_dependencies(&project_root)
    {
        all_entities.extend(cargo_deps);
    }
    if let Ok(npm_deps) =
        ast_pipeline::npm_dep_extractor::extract_npm_dependencies(&project_root)
    {
        all_entities.extend(npm_deps);
    }

    if all_entities.is_empty() {
        tracing::debug!("[KGRAPH] No AST entities found to index");
        return;
    }

    match db.load_entities(&all_entities).await {
        Ok(count) => {
            tracing::info!(count, "[KGRAPH] AST graph populated at session start");
        }
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to load AST entities: {e}");
        }
    }
}

/// Extract learnings from a compaction DAG summary.
///
/// Scans for structural patterns that indicate decisions, conventions, and
/// constraints in the DAG text, then loads them into the Learnings graph.
///
/// Called at compaction boundaries (after inject_summary applies the DAG).
/// Does NOT call an LLM — processes the DAG text structurally.
pub async fn extract_learnings_from_dag(dag_text: &str) {
    if dag_text.trim().is_empty() {
        return;
    }

    let db = match registry::get_graph(registry::LEARNINGS_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to open Learnings graph for extraction: {e}");
            return;
        }
    };

    let entities = extract_structural_learnings_from_dag(dag_text);

    if entities.is_empty() {
        tracing::debug!("[KGRAPH] No structural learnings found in DAG summary");
        return;
    }

    match db.load_entities(&entities).await {
        Ok(count) => {
            tracing::info!(count, "[KGRAPH] Learnings extracted from DAG summary");
        }
        Err(e) => {
            tracing::warn!("[KGRAPH] Failed to load learnings entities: {e}");
        }
    }
}

/// Structurally extract Learning entities from DAG summary text.
///
/// Scans for patterns that indicate decisions, conventions, and constraints
/// without requiring an LLM call. Produces Learning nodes with appropriate
/// categories. Uses keyword matching on each line for zero-cost extraction.
pub fn extract_structural_learnings_from_dag(dag_text: &str) -> Vec<graph_entities::GraphEntity> {
    use chrono::Utc;
    use serde_json::{Map, Value};

    let mut entities = Vec::new();
    let now = Utc::now().to_rfc3339();

    for line in dag_text.lines() {
        let trimmed = line.trim();

        // Skip empty lines and very short lines
        if trimmed.len() < 20 {
            continue;
        }

        // Look for decision markers
        let (category, slug_prefix) = if trimmed.contains("decided to ")
            || trimmed.contains("decision:")
            || trimmed.starts_with("Decision:")
        {
            ("decision", "dag-decision")
        } else if trimmed.contains("convention:")
            || trimmed.contains("Convention:")
            || trimmed.contains("always use ")
            || trimmed.contains("never use ")
        {
            ("convention", "dag-convention")
        } else if trimmed.contains("constraint:")
            || trimmed.contains("Constraint:")
            || trimmed.contains("limitation:")
            || trimmed.contains("cannot ")
            || trimmed.contains("must not ")
        {
            ("constraint", "dag-constraint")
        } else {
            continue;
        };

        // Build a slug from the first 50 chars
        let slug_text: String = trimmed
            .chars()
            .take(50)
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .collect::<String>()
            .to_lowercase()
            .replace(' ', "-");
        let slug = format!("{slug_prefix}-{slug_text}");

        let title: String = trimmed.chars().take(100).collect();

        let mut props = Map::new();
        props.insert("slug".to_string(), Value::String(slug.clone()));
        props.insert("title".to_string(), Value::String(title));
        props.insert(
            "content".to_string(),
            Value::String(trimmed.to_string()),
        );
        props.insert(
            "category".to_string(),
            Value::String(category.to_string()),
        );
        props.insert(
            "confidence".to_string(),
            Value::String("medium".to_string()),
        );
        props.insert("firstSeen".to_string(), Value::String(now.clone()));
        props.insert("lastSeen".to_string(), Value::String(now.clone()));
        props.insert(
            "mentionCount".to_string(),
            Value::Number(1.into()),
        );

        entities.push(graph_entities::GraphEntity::Node {
            node_type: "Learning".to_string(),
            slug,
            properties: props,
        });
    }

    // Enforce volume limit
    if entities.len() > 20 {
        entities.truncate(20);
    }

    entities
}

pub mod ast_dispatch;
pub mod ast_pipeline;
pub mod database;
pub mod dispatch_helpers;
pub mod graph_entities;
pub mod learnings_context;
pub mod learnings_dispatch;
pub mod learnings_extraction;
pub mod llm_response_parser;
pub mod registry;
