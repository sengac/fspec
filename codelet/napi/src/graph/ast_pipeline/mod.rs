//! AST Extraction Pipeline
//!
//! Walks a codebase and extracts AST entities (Functions, Types, Imports)
//! into `GraphEntity` values for the AST Connection Graph.
//!
//! Delegates to per-language extractors:
//! - `ast_ts_extractor`: TypeScript/JavaScript
//! - `ast_rust_extractor`: Rust
//!
//! Uses `ignore::WalkBuilder` for `.gitignore`-aware file walking.
//! All extracted entities are batched before loading.

use std::path::Path;

use super::graph_entities::GraphEntity;

pub mod ast_rust_extractor;
pub mod ast_ts_extractor;
pub mod cargo_dep_extractor;
pub mod npm_dep_extractor;
pub(crate) mod helpers;

/// Supported source file extensions for AST extraction.
const SUPPORTED_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx", "mjs", "mts", "rs"];

/// Directories to always skip even without .gitignore.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    ".git",
    ".fspec",
    "__pycache__",
];

/// Extract AST entities from a single file.
///
/// Determines the language from the file extension and delegates
/// to the appropriate extractor. Returns all entities (nodes + edges)
/// for this file.
pub fn extract_file(file_path: &Path, project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let ext = file_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let rel_path = file_path
        .strip_prefix(project_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let source = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Failed to read {}: {e}", file_path.display()))?;

    match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts" => {
            ast_ts_extractor::extract_typescript(&source, &rel_path)
        }
        "rs" => ast_rust_extractor::extract_rust(&source, &rel_path),
        _ => Ok(vec![]), // Unsupported language — skip
    }
}

/// Walk a project directory and extract all AST entities.
///
/// Respects `.gitignore` and skips common non-source directories
/// (node_modules, target, dist, .git). Returns a flat list of all
/// entities across all files, suitable for batch loading.
pub fn walk_and_extract(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let walker = ignore::WalkBuilder::new(project_root)
        .hidden(true) // Skip hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(false)
        .git_exclude(false)
        .build();

    let mut all_entities = Vec::new();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();

        // Explicitly skip well-known directories even without .gitignore
        let rel = path.strip_prefix(project_root).unwrap_or(path);
        if rel
            .components()
            .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
        {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Only process supported language files
        if !SUPPORTED_EXTENSIONS.contains(&ext) {
            continue;
        }

        match extract_file(path, project_root) {
            Ok(entities) => all_entities.extend(entities),
            Err(e) => {
                tracing::warn!(?path, error = %e, "failed to extract AST from file");
            }
        }
    }

    Ok(all_entities)
}
