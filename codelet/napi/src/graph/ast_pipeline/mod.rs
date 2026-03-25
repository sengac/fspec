//! AST Extraction Pipeline
//!
//! Walks a codebase and extracts AST entities (Functions, Types, Imports)
//! into `GraphEntity` values for the AST Connection Graph.
//!
//! Delegates to per-language extractors:
//! - `ast_ts_extractor`: TypeScript/JavaScript
//! - `ast_rust_extractor`: Rust
//! - `ast_python_extractor`: Python
//! - `ast_go_extractor`: Go
//! - `ast_java_extractor`: Java
//! - `ast_c_extractor`: C
//! - `ast_cpp_extractor`: C++
//! - `ast_csharp_extractor`: C#
//! - `ast_ruby_extractor`: Ruby
//! - `ast_kotlin_extractor`: Kotlin
//! - `ast_swift_extractor`: Swift
//! - `ast_scala_extractor`: Scala
//! - `ast_php_extractor`: PHP
//!
//! Uses `ignore::WalkBuilder` for `.gitignore`-aware file walking.
//! All extracted entities are batched before loading.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::graph_entities::GraphEntity;

pub mod ast_c_extractor;
pub mod ast_cpp_extractor;
pub mod ast_csharp_extractor;
pub mod ast_go_extractor;
pub mod ast_java_extractor;
pub mod ast_kotlin_extractor;
pub mod ast_php_extractor;
pub mod ast_python_extractor;
pub mod ast_ruby_extractor;
pub mod ast_rust_extractor;
pub mod ast_scala_extractor;
pub mod ast_swift_extractor;
pub mod ast_ts_extractor;
pub mod cargo_dep_extractor;
pub mod composer_dep_extractor;
pub mod csproj_dep_extractor;
pub mod gemfile_dep_extractor;
pub mod gomod_dep_extractor;
pub mod java_dep_extractor;
pub mod npm_dep_extractor;
pub mod pip_dep_extractor;
pub mod sbt_dep_extractor;
pub mod swift_dep_extractor;
pub(crate) mod helpers;
pub(crate) mod edge_helpers;

/// Supported source file extensions for AST extraction.
const SUPPORTED_EXTENSIONS: &[&str] = &[
    "ts", "tsx", "js", "jsx", "mjs", "mts", // TypeScript/JavaScript
    "rs",                                     // Rust
    "py", "pyi",                              // Python
    "go",                                     // Go
    "java",                                   // Java
    "c", "h",                                 // C (h files may be C or C++)
    "cpp", "cc", "cxx", "hpp",                // C++
    "cs",                                     // C#
    "rb", "gemspec",                          // Ruby
    "kt", "kts",                              // Kotlin
    "swift",                                  // Swift
    "scala", "sc",                            // Scala
    "php",                                    // PHP
];

/// Directories to always skip even without .gitignore.
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "dist",
    ".git",
    ".fspec",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    "venv",
    ".venv",
    "vendor",
    "Pods",
    ".gradle",
    "build",
];

/// Extract AST entities from a single file.
///
/// Determines the language from the file extension and delegates
/// to the appropriate extractor. Returns all entities (nodes + edges)
/// for this file.
///
/// The `known_files` set contains relative paths of all source files in
/// the project (used for barrel-import resolution in TypeScript). Pass
/// an empty set when extracting a single file in isolation.
///
/// Panics from ast-grep or extractor code are caught via `catch_unwind`
/// and converted to `Err`, so a malformed source file never crashes the
/// indexing pipeline.
pub fn extract_file(
    file_path: &Path,
    project_root: &Path,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
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

    // Wrap extraction in catch_unwind to turn panics (e.g. from ast-grep
    // or byte-level string slicing on non-ASCII source) into Err values.
    let known_files_clone = known_files.clone();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match ext {
        "ts" | "tsx" | "js" | "jsx" | "mjs" | "mts" => {
            ast_ts_extractor::extract_typescript(&source, &rel_path, &known_files_clone)
        }
        "rs" => ast_rust_extractor::extract_rust(&source, &rel_path, &known_files_clone),
        "py" | "pyi" => ast_python_extractor::extract_python(&source, &rel_path, &known_files_clone),
        "go" => ast_go_extractor::extract_go(&source, &rel_path, &known_files_clone),
        "java" => ast_java_extractor::extract_java(&source, &rel_path, &known_files_clone),
        "c" => ast_c_extractor::extract_c(&source, &rel_path, &known_files_clone),
        "h" => {
            // Heuristic: if the .h file looks like C++, parse as C++
            if ast_cpp_extractor::is_cpp_header(&source) {
                ast_cpp_extractor::extract_cpp(&source, &rel_path, &known_files_clone)
            } else {
                ast_c_extractor::extract_c(&source, &rel_path, &known_files_clone)
            }
        }
        "cpp" | "cc" | "cxx" | "hpp" => ast_cpp_extractor::extract_cpp(&source, &rel_path, &known_files_clone),
        "cs" => ast_csharp_extractor::extract_csharp(&source, &rel_path, &known_files_clone),
        "rb" | "gemspec" => ast_ruby_extractor::extract_ruby(&source, &rel_path, &known_files_clone),
        "kt" | "kts" => ast_kotlin_extractor::extract_kotlin(&source, &rel_path, &known_files_clone),
        "swift" => ast_swift_extractor::extract_swift(&source, &rel_path, &known_files_clone),
        "scala" | "sc" => ast_scala_extractor::extract_scala(&source, &rel_path, &known_files_clone),
        "php" => ast_php_extractor::extract_php(&source, &rel_path, &known_files_clone),
        _ => Ok(vec![]), // Unsupported language — skip
    }));

    match result {
        Ok(inner) => inner,
        Err(panic_payload) => {
            let msg = panic_payload_to_string(&panic_payload);
            Err(format!(
                "AST extraction panicked for {}: {msg}",
                file_path.display()
            ))
        }
    }
}

/// Convert a `catch_unwind` panic payload into a human-readable string.
fn panic_payload_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "unknown panic".to_string()
    }
}

/// Walk a project directory and extract all AST entities.
///
/// Respects `.gitignore` and skips common non-source directories
/// (node_modules, target, dist, .git). Returns a flat list of all
/// entities across all files, suitable for batch loading.
///
/// Uses a two-phase approach:
/// 1. **Collect** all source file paths (cheap directory walk)
/// 2. **Extract** each file with knowledge of all paths (enables barrel-import resolution)
///
/// When `respect_gitignore` is false, `.gitignore` rules are skipped so
/// external repos under gitignored directories can be indexed.
pub fn walk_and_extract(project_root: &Path, respect_gitignore: bool) -> Result<Vec<GraphEntity>, String> {
    // Phase 1: Collect all source file paths for import resolution context
    let mut source_files: Vec<std::path::PathBuf> = Vec::new();
    let walker = ignore::WalkBuilder::new(project_root)
        .hidden(true)
        .git_ignore(respect_gitignore)
        .git_global(false)
        .git_exclude(false)
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.into_path();

        let rel = path.strip_prefix(project_root).unwrap_or(&path);
        if rel
            .components()
            .any(|c| SKIP_DIRS.contains(&c.as_os_str().to_str().unwrap_or("")))
        {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !SUPPORTED_EXTENSIONS.contains(&ext) {
            continue;
        }

        source_files.push(path);
    }

    // Build known files set (relative paths with forward slashes)
    let known_files: HashSet<String> = source_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(project_root)
                .ok()
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        })
        .collect();

    // Phase 2: Extract each file with the full known-files context
    let mut all_entities = Vec::new();

    for path in &source_files {
        match extract_file(path, project_root, &known_files) {
            Ok(entities) => all_entities.extend(entities),
            Err(e) => {
                tracing::warn!(?path, error = %e, "failed to extract AST from file");
            }
        }
    }

    Ok(deduplicate_entities(all_entities))
}

/// Deduplicate graph entities by `(node_type, slug)` and prune dangling edges.
///
/// When two Node entities share the same `(node_type, slug)` key, the one
/// with **more properties** is kept (full File node wins over stub).
///
/// After deduplication, edges whose `from_slug` or `to_slug` don't match
/// any known node slug are dropped. This prevents `@key` constraint violations
/// in nanograph when cross-file edges reference aliased imports or functions
/// that weren't extracted (e.g., arrow functions, class methods).
pub fn deduplicate_entities(entities: Vec<GraphEntity>) -> Vec<GraphEntity> {
    // Track seen nodes by (node_type, slug) → index into deduped_nodes
    let mut node_map: HashMap<(String, String), usize> = HashMap::new();
    let mut deduped_nodes: Vec<GraphEntity> = Vec::new();
    let mut edges: Vec<GraphEntity> = Vec::new();

    for entity in entities {
        match &entity {
            GraphEntity::Node {
                node_type,
                slug,
                properties,
            } => {
                let key = (node_type.clone(), slug.clone());
                if let Some(&existing_idx) = node_map.get(&key) {
                    // Keep the node with more properties (full > stub)
                    if let GraphEntity::Node {
                        properties: ref existing_props,
                        ..
                    } = deduped_nodes[existing_idx]
                    {
                        if properties.len() > existing_props.len() {
                            deduped_nodes[existing_idx] = entity;
                        }
                    }
                } else {
                    node_map.insert(key, deduped_nodes.len());
                    deduped_nodes.push(entity);
                }
            }
            GraphEntity::Edge { .. } => {
                edges.push(entity);
            }
        }
    }

    // Build a typed slug map: slug → set of node_types that slug belongs to.
    // This lets us validate that edge endpoints match the schema-expected types
    // (e.g. Calls: Function→Function, TypeRef: Function→Type).
    let mut slug_types: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for node in &deduped_nodes {
        if let GraphEntity::Node {
            node_type, slug, ..
        } = node
        {
            slug_types
                .entry(slug.clone())
                .or_default()
                .insert(node_type.clone());
        }
    }

    // Schema-expected target types for each edge kind.
    // Calls: Function→Function, TypeRef: Function→Type,
    // Contains: File→Function, ContainsType: File→Type,
    // Imports: File→File, DependsOn: File→Dependency,
    // Implements: Type→Type, Extends: Type→Type.
    fn expected_target_type(edge_type: &str) -> Option<&'static str> {
        match edge_type {
            "Calls" => Some("Function"),
            "TypeRef" => Some("Type"),
            "Contains" => Some("Function"),
            "ContainsType" => Some("Type"),
            "Imports" => Some("File"),
            "DependsOn" => Some("Dependency"),
            "Implements" | "Extends" => Some("Type"),
            _ => None,
        }
    }

    fn expected_source_type(edge_type: &str) -> Option<&'static str> {
        match edge_type {
            "Calls" => Some("Function"),
            "TypeRef" => Some("Function"),
            "Contains" | "ContainsType" | "Imports" | "DependsOn" => Some("File"),
            "Implements" | "Extends" => Some("Type"),
            _ => None,
        }
    }

    // Only keep edges whose both endpoints exist AND match schema-expected types
    let mut pruned = 0usize;
    for edge in edges {
        if let GraphEntity::Edge {
            ref edge_type,
            ref from_slug,
            ref to_slug,
            ..
        } = edge
        {
            let from_exists = slug_types.contains_key(from_slug.as_str());
            let to_exists = slug_types.contains_key(to_slug.as_str());

            if !from_exists || !to_exists {
                pruned += 1;
                continue;
            }

            // Validate source node type matches schema
            if let Some(expected_src) = expected_source_type(edge_type) {
                if let Some(types) = slug_types.get(from_slug.as_str()) {
                    if !types.contains(expected_src) {
                        pruned += 1;
                        continue;
                    }
                }
            }

            // Validate target node type matches schema
            if let Some(expected_tgt) = expected_target_type(edge_type) {
                if let Some(types) = slug_types.get(to_slug.as_str()) {
                    if !types.contains(expected_tgt) {
                        pruned += 1;
                        continue;
                    }
                }
            }

            deduped_nodes.push(edge);
        }
    }

    if pruned > 0 {
        tracing::debug!(pruned, "pruned dangling edges with unknown target nodes");
    }

    deduped_nodes
}
