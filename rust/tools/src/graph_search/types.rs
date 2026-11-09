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
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        query: String,
        entity_type: Option<String>,
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
        /// Glob filter to scope results to matching file paths (e.g. "src/tui/**/*.tsx").
        path: Option<String>,
        /// Search mode: "name" (default — name/slug/path/qualifiedName), "content" (source/docstring), "all" (every field).
        search_mode: Option<String>,
        /// Filter by decorator/annotation (case-insensitive, strips leading @/#[ for cross-language matching).
        decorator: Option<String>,
        /// Filter by parameter name (case-insensitive contains match on comma-separated parameter names).
        parameter: Option<String>,
    },
    /// Get AST graph neighbors of a code entity node.
    AstNeighbors {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        node_id: String,
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        depth: Option<usize>,
        edge_types: Option<Vec<String>>,
    },
    /// Get AST codebase statistics (node/edge type counts).
    AstStats {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
    },

    /// Index a root (the session's working directory by default) into its
    /// own code graph.
    AstIndex {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Not accepted: `ast_index` takes the directory to index as `root`;
        /// `path` is the query glob filter. Kept so a stray `path` fails
        /// loudly instead of being ignored.
        path: Option<String>,
        /// When `true`, deletes the existing on-disk database and clears
        /// the in-memory graph singleton before re-indexing.
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_bool")]
        reset: Option<bool>,
        /// When `true`, only re-extracts files whose modification time has
        /// changed since the last index.
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_bool")]
        incremental: Option<bool>,
    },

    /// Detect dead code: orphan files, uncalled functions, unreferenced types.
    AstDeadCode {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        entity_type: Option<String>,
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
        /// Glob filter to scope results to matching file paths (e.g. "src/tui/**/*.tsx").
        path: Option<String>,
    },

    /// Find call chain(s) between two functions via multi-hop CALLS edge traversal.
    AstCallChain {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Source function slug or name.
        from: String,
        /// Target function slug or name.
        to: String,
        /// Maximum BFS depth (default 5).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        max_depth: Option<u32>,
    },

    /// Find all transitive callers of a function via multi-hop incoming CALLS edges.
    AstCallers {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Function slug or name to find callers of.
        node_id: String,
        /// Maximum BFS depth (default 5).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        max_depth: Option<u32>,
        /// Maximum number of results (default 50).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
    },

    /// Find all transitive callees of a function via multi-hop outgoing CALLS edges.
    AstCallees {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Function slug or name to find callees of.
        node_id: String,
        /// Maximum BFS depth (default 5).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        max_depth: Option<u32>,
        /// Maximum number of results (default 50).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
    },

    /// Get the full inheritance hierarchy for a type.
    AstHierarchy {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Type slug to find hierarchy for.
        node_id: String,
        /// Maximum BFS depth for parent/child traversal (default 3).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        depth: Option<u32>,
        /// Whether to include methods (default true).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_bool")]
        include_methods: Option<bool>,
    },

    /// Query cyclomatic complexity of functions in the codebase.
    AstComplexity {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// Function slug for single-function lookup. Omit for top-N mode.
        node_id: Option<String>,
        /// Maximum results to return (default 20, only for top-N mode).
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
        /// Only return functions with complexity >= this value.
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_u32")]
        min_threshold: Option<u32>,
        /// Glob filter to scope results to matching file paths.
        path: Option<String>,
    },

    /// Export the AST graph to a portable `.astbundle` ZIP archive.
    AstExport {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// File path for the output `.astbundle` file.
        output_path: String,
    },

    /// Import a `.astbundle` ZIP archive into the AST graph.
    AstImport {
        /// KGRAPH-073: directory whose code graph to use; a relative root joins
        /// the session's working directory, which is also the default.
        root: Option<String>,
        /// File path to the `.astbundle` file to import.
        input_path: String,
        /// Import mode: `"overwrite"` (default) or `"merge"`.
        merge_mode: Option<String>,
    },

    // ── Learnings Graph Actions ──────────────────────────────────
    /// Search Learnings entities (learnings, decisions, conventions) by text/category.
    LearningsSearch {
        query: String,
        category: Option<String>,
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
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
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_f32")]
        min_strength: Option<f32>,
        #[serde(default, deserialize_with = "crate::serde_coerce::deser_option_usize")]
        limit: Option<usize>,
    },
}

/// Top-level args struct for the GraphSearch tool.
#[derive(Debug, Clone, Deserialize)]
pub struct GraphSearchArgs {
    #[serde(flatten)]
    pub action: GraphSearchAction,
}
