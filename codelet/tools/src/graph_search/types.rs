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
        node_id: String,
        depth: Option<usize>,
        edge_types: Option<Vec<String>>,
    },
    /// Get AST codebase statistics (node/edge type counts).
    AstStats,

    /// Index the project codebase into the AST graph.
    ///
    /// Walks the project directory, extracts functions/types/imports via
    /// ast-grep, extracts dependencies from Cargo.toml/package.json,
    /// and loads everything into the AST code graph.
    ///
    /// When `path` is provided, indexes only that directory (ignoring
    /// `.gitignore` rules so external repos can be indexed). Falls back
    /// to the current working directory when omitted.
    AstIndex {
        /// Optional directory to index. When omitted, indexes from cwd.
        /// When provided, `.gitignore` is disabled so external/vendored
        /// repos under gitignored directories can be indexed.
        path: Option<String>,
        /// When `true`, deletes the existing on-disk database and clears
        /// the in-memory graph singleton before re-indexing. Required
        /// after schema changes that make the existing database incompatible.
        reset: Option<bool>,
        /// When `true`, only re-extracts files whose modification time has
        /// changed since the last index. Unchanged file entities are reused
        /// from the existing graph. Falls back to full extraction when no
        /// prior index exists or when >50% of files have changed.
        incremental: Option<bool>,
    },

    /// Detect dead code: orphan files, uncalled functions, unreferenced types.
    ///
    /// Uses nanograph `not { }` anti-join queries on the AST graph.
    /// Excludes test files and external stubs by default.
    AstDeadCode {
        entity_type: Option<String>,
        limit: Option<usize>,
        /// Glob filter to scope results to matching file paths (e.g. "src/tui/**/*.tsx").
        path: Option<String>,
    },

    /// Find call chain(s) between two functions via multi-hop CALLS edge traversal.
    ///
    /// Uses BFS over single-hop nanograph queries to find the shortest paths.
    /// Returns chains ordered by length (shortest first), limited to 20.
    AstCallChain {
        /// Source function slug or name.
        from: String,
        /// Target function slug or name.
        to: String,
        /// Maximum BFS depth (default 5).
        max_depth: Option<u32>,
    },

    /// Find all transitive callers of a function via multi-hop incoming CALLS edges.
    ///
    /// Returns a flat list of functions annotated with hop distance from the target.
    /// Uses BFS over reversed adjacency list from KGRAPH-060 infrastructure.
    AstCallers {
        /// Function slug or name to find callers of.
        node_id: String,
        /// Maximum BFS depth (default 5).
        max_depth: Option<u32>,
        /// Maximum number of results (default 50).
        limit: Option<usize>,
    },

    /// Find all transitive callees of a function via multi-hop outgoing CALLS edges.
    ///
    /// Returns a flat list of functions annotated with hop distance from the source.
    /// Uses BFS over forward adjacency list from KGRAPH-060 infrastructure.
    AstCallees {
        /// Function slug or name to find callees of.
        node_id: String,
        /// Maximum BFS depth (default 5).
        max_depth: Option<u32>,
        /// Maximum number of results (default 50).
        limit: Option<usize>,
    },

    /// Get the full inheritance hierarchy for a type (parents, children, interfaces, methods).
    ///
    /// Uses iterative BFS over Extends edges (parents upward, children downward)
    /// and single-hop Implements edges for interfaces.
    /// Methods are approximated as functions in the same file as the type.
    AstHierarchy {
        /// Type slug to find hierarchy for.
        node_id: String,
        /// Maximum BFS depth for parent/child traversal (default 3).
        depth: Option<u32>,
        /// Whether to include methods (default true).
        include_methods: Option<bool>,
    },

    /// Query cyclomatic complexity of functions in the codebase.
    ///
    /// Two modes:
    /// - **Single function**: Pass `node_id` to get complexity of one function.
    /// - **Top-N**: Omit `node_id` to get most complex functions sorted DESC.
    ///
    /// CGC equivalent: `get_cyclomatic_complexity()` / `find_most_complex_functions()`.
    AstComplexity {
        /// Function slug for single-function lookup. Omit for top-N mode.
        node_id: Option<String>,
        /// Maximum results to return (default 20, only for top-N mode).
        limit: Option<usize>,
        /// Only return functions with complexity >= this value.
        min_threshold: Option<u32>,
        /// Glob filter to scope results to matching file paths.
        path: Option<String>,
    },

    /// Export the AST graph to a portable `.astbundle` ZIP archive.
    ///
    /// The bundle contains all nodes, edges, metadata, and the schema source.
    /// Can be shared across sessions, teams, or machines and imported with
    /// `ast_import` to avoid re-indexing.
    ///
    /// CGC equivalent: `export_to_bundle()` with `.cgc` ZIP format.
    AstExport {
        /// File path for the output `.astbundle` file.
        output_path: String,
    },

    /// Import a `.astbundle` ZIP archive into the AST graph.
    ///
    /// Validates schema compatibility before loading. Supports two modes:
    /// - `"overwrite"` (default) — replaces all existing data
    /// - `"merge"` — upserts via slug-based key matching
    ///
    /// CGC equivalent: `import_from_bundle()` with `clear_existing` flag.
    AstImport {
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
