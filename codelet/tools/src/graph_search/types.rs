//! GraphSearch Tool — Types
//!
//! Defines the action enum and args struct for the GraphSearch tool.
//! Uses serde-tagged enum pattern matching SessionSearch.

use serde::Deserialize;

/// Discriminated union for GraphSearch actions.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action_type", rename_all = "snake_case")]
pub enum GraphSearchAction {
    /// Full-text search across concept names and summaries.
    Search {
        query: String,
        category: Option<String>,
        limit: Option<usize>,
    },
    /// Get neighbors of a node within N hops.
    Neighbors {
        node_id: String,
        depth: Option<usize>,
        edge_types: Option<Vec<String>>,
    },
    /// Find shortest path between two nodes.
    Path {
        from: String,
        to: String,
        max_hops: Option<usize>,
    },
    /// Find concepts related to a topic by co-occurrence.
    Related {
        topic: String,
        min_strength: Option<f32>,
        limit: Option<usize>,
    },
    /// Query decisions by domain or status.
    Decisions {
        domain: Option<String>,
        status: Option<String>,
        since: Option<String>,
    },
    /// Get session/turn history for a concept.
    History {
        concept: String,
        limit: Option<usize>,
    },
    /// Get node/edge type counts.
    Stats,
    /// Trigger indexing of session data into the graph.
    Index {
        scope: Option<String>,
    },
}

/// Top-level args struct for the GraphSearch tool.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphSearchArgs {
    #[serde(flatten)]
    pub action: GraphSearchAction,
}
