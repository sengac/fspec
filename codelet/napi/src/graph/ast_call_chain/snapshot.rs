//! Pre-fetched graph data to avoid redundant database queries.

use crate::graph::ast_dispatch::AST_QUERIES;
use crate::graph::database::GraphDatabase;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

/// Pre-fetched graph data to avoid redundant database queries.
///
/// A single `dispatch_ast_call_chain` call needs function metadata
/// multiple times (existence checks, adjacency building, enrichment).
/// This struct fetches `all_functions` and `all_files` once and
/// reuses the results, including file path resolution.
pub struct GraphSnapshot {
    /// Set of all known function slugs for O(1) existence checks.
    known_slugs: HashSet<String>,
    /// Full function metadata keyed by slug, for enriching results.
    func_metadata: HashMap<String, Value>,
    /// File slug → file path mapping for resolving function file paths.
    file_paths: HashMap<String, String>,
}

impl GraphSnapshot {
    /// Load all function and file data from the graph in bulk queries.
    pub async fn load(db: &GraphDatabase) -> Self {
        let mut known_slugs = HashSet::new();
        let mut func_metadata = HashMap::new();
        let mut file_paths = HashMap::new();

        // Load all files to build slug→path mapping
        if let Ok(Value::Array(files)) =
            db.query_with_source(AST_QUERIES, "all_files", None).await
        {
            for file in files {
                if let (Some(slug), Some(path)) = (
                    file.get("slug").and_then(|v| v.as_str()),
                    file.get("path").and_then(|v| v.as_str()),
                ) {
                    file_paths.insert(slug.to_string(), path.to_string());
                }
            }
        }

        // Load all functions
        if let Ok(Value::Array(fns)) =
            db.query_with_source(AST_QUERIES, "all_functions", None).await
        {
            for func in fns {
                if let Some(slug) = func.get("slug").and_then(|v| v.as_str()) {
                    known_slugs.insert(slug.to_string());
                    func_metadata.insert(slug.to_string(), func);
                }
            }
        }

        Self { known_slugs, func_metadata, file_paths }
    }

    /// Check if a function slug exists in the graph (O(1) lookup).
    pub fn function_exists(&self, slug: &str) -> bool {
        self.known_slugs.contains(slug)
    }

    /// Get the set of all known function slugs.
    pub fn known_slugs(&self) -> &HashSet<String> {
        &self.known_slugs
    }

    /// Get function metadata by slug, falling back to a stub.
    pub fn get_metadata(&self, slug: &str) -> Value {
        self.func_metadata
            .get(slug)
            .cloned()
            .unwrap_or_else(|| serde_json::json!({ "slug": slug }))
    }

    /// Resolve the file path for a function slug by extracting its
    /// file slug prefix (everything before `::`) and looking up the
    /// corresponding file path.
    pub fn get_file_path(&self, fn_slug: &str) -> Option<&str> {
        let file_slug = fn_slug.split("::").next()?;
        self.file_paths.get(file_slug).map(|s| s.as_str())
    }
}
