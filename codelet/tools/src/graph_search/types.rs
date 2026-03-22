//! GraphSearch Tool — Types
//!
//! Defines the action enum and args struct for the GraphSearch tool.
//! Uses serde-tagged enum pattern matching SessionSearch.
//!
//! Only AST and Learnings graph actions are supported (dual-graph architecture).

use serde::Deserialize;

/// Discriminated union for GraphSearch actions.
///
/// Supports two graph databases:
/// - AST Code Graph (AstSearch, AstNeighbors, AstStats)
/// - Learnings Graph (LearningsSearch, LearningsDecisions, LearningsStats, LearningsRelated)
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum GraphSearchAction {
    // ── AST Graph Actions ────────────────────────────────────

    /// Search AST code entities (functions, types, files) by name/pattern.
    AstSearch {
        query: String,
        entity_type: Option<String>,
        limit: Option<usize>,
    },
    /// Get AST graph neighbors of a code entity node.
    AstNeighbors {
        node_id: String,
        depth: Option<usize>,
        edge_types: Option<Vec<String>>,
    },
    /// Get AST codebase statistics (node/edge type counts).
    AstStats,

    // ── Learnings Graph Actions ──────────────────────────────────

    /// Search Learnings entities (learnings, decisions, conventions) by text/category.
    LearningsSearch {
        query: String,
        category: Option<String>,
        limit: Option<usize>,
    },
    /// Query Decision nodes filtered by domain and/or status.
    LearningsDecisions {
        domain: Option<String>,
        status: Option<String>,
    },
    /// Get Learnings graph statistics (node/edge type counts).
    LearningsStats,
    /// Find learnings related to a topic via RelatesTo edges.
    LearningsRelated {
        topic: String,
        min_strength: Option<f32>,
        limit: Option<usize>,
    },
}

/// Top-level args struct for the GraphSearch tool.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphSearchArgs {
    #[serde(flatten)]
    pub action: GraphSearchAction,
}
