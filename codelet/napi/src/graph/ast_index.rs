//! AST Index Dispatch
//!
//! Handles the `ast_index` action: walks the project directory, extracts
//! AST entities (functions, types, imports, dependencies), and batch-loads
//! them into the AST code graph.
//!
//! Extracted from `ast_dispatch.rs` for file size compliance.

use crate::graph::dispatch_helpers::format_graph_stats;

/// Index (or re-index) the project codebase into the AST graph.
///
/// Walks the project directory, extracts functions/types/imports via ast-grep,
/// extracts dependencies from Cargo.toml/package.json, and batch-loads
/// everything into the AST code graph. Idempotent — uses nanograph upsert
/// semantics so repeated calls are safe.
///
/// When `custom_path` is provided, indexes only that directory with
/// `.gitignore` disabled. Falls back to `cwd` when `None`.
///
/// When `reset` is `true`, deletes the existing on-disk database and clears
/// the in-memory graph singleton before re-indexing. This is required after
/// schema changes that make the existing database incompatible.
pub async fn dispatch_ast_index(custom_path: Option<&str>, reset: bool) -> String {
    let (project_root, respect_gitignore) = if let Some(p) = custom_path {
        let path = std::path::Path::new(p);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            match std::env::current_dir() {
                Ok(cwd) => cwd.join(path),
                Err(e) => {
                    return serde_json::json!({
                        "action": "ast_index",
                        "error": format!("Failed to get current directory: {e}"),
                    })
                    .to_string();
                }
            }
        };
        if !resolved.is_dir() {
            return serde_json::json!({
                "action": "ast_index",
                "error": format!("Path is not a directory: {}", resolved.display()),
            })
            .to_string();
        }
        (resolved, false)
    } else {
        match std::env::current_dir() {
            Ok(p) => (p, true),
            Err(e) => {
                return serde_json::json!({
                    "action": "ast_index",
                    "error": format!("Failed to get current directory: {e}"),
                })
                .to_string();
            }
        }
    };

    // If reset requested, delete on-disk data and clear in-memory singleton
    if reset {
        let (db_path, _schema) =
            match super::registry::resolve_graph_config(super::registry::AST_CODE_GRAPH) {
                Ok(config) => config,
                Err(e) => {
                    return serde_json::json!({
                        "action": "ast_index",
                        "error": format!("Failed to resolve graph config for reset: {e}"),
                    })
                    .to_string();
                }
            };

        match super::registry::delete_graph_data(super::registry::AST_CODE_GRAPH, &db_path) {
            Ok(deleted) => {
                tracing::info!(
                    deleted,
                    "AST graph reset: on-disk data deleted and in-memory cache cleared"
                );
            }
            Err(e) => {
                return serde_json::json!({
                    "action": "ast_index",
                    "error": format!("Failed to reset graph: {e}"),
                })
                .to_string();
            }
        }
    }

    let db = match super::registry::get_graph(super::registry::AST_CODE_GRAPH).await {
        Ok(db) => db,
        Err(e) => {
            return serde_json::json!({
                "action": "ast_index",
                "error": e,
            })
            .to_string();
        }
    };

    // Walk codebase and extract AST entities
    let mut all_entities =
        match super::ast_pipeline::walk_and_extract(&project_root, respect_gitignore) {
            Ok(entities) => entities,
            Err(e) => {
                return serde_json::json!({
                    "action": "ast_index",
                    "error": format!("AST extraction failed: {e}"),
                })
                .to_string();
            }
        };

    // Extract dependencies from all supported package managers
    extract_all_dependencies(&project_root, &mut all_entities);

    // Deduplicate after merging dep-extractor results with AST entities.
    // walk_and_extract already deduplicates internally, but dep extractors
    // may emit File nodes that overlap (e.g. Package.swift is both a Swift
    // source file and the SPM manifest), so we run a final pass.
    let all_entities = super::ast_pipeline::deduplicate_entities(all_entities);

    let entity_count = all_entities.len();
    if entity_count == 0 {
        return serde_json::json!({
            "action": "ast_index",
            "entities_loaded": 0,
            "message": "No source files found to index",
        })
        .to_string();
    }

    // Batch-load into graph (overwrite to remove stale entities from prior index)
    match db.load_entities_overwrite(&all_entities).await {
        Ok(loaded) => {
            tracing::info!(loaded, "AST index complete — entities loaded into graph");
            // Return fresh stats after indexing
            let stats = format_graph_stats(&db, "ast_index");
            // Merge stats with load count
            if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&stats) {
                if let Some(obj) = parsed.as_object_mut() {
                    obj.insert(
                        "entities_loaded".to_string(),
                        serde_json::Value::Number(loaded.into()),
                    );
                }
                parsed.to_string()
            } else {
                stats
            }
        }
        Err(e) => {
            serde_json::json!({
                "action": "ast_index",
                "error": format!("Failed to load entities into graph: {e}"),
            })
            .to_string()
        }
    }
}

/// Extract dependencies from all supported package managers into the entity list.
fn extract_all_dependencies(
    project_root: &std::path::Path,
    all_entities: &mut Vec<super::graph_entities::GraphEntity>,
) {
    let extractors: &[(&str, fn(&std::path::Path) -> Result<Vec<super::graph_entities::GraphEntity>, String>)] = &[
        ("Cargo", super::ast_pipeline::cargo_dep_extractor::extract_cargo_dependencies),
        ("NPM", super::ast_pipeline::npm_dep_extractor::extract_npm_dependencies),
        ("Python", super::ast_pipeline::pip_dep_extractor::extract_python_dependencies),
        ("Go", super::ast_pipeline::gomod_dep_extractor::extract_go_dependencies),
        ("Java", super::ast_pipeline::java_dep_extractor::extract_java_dependencies),
        ("Composer", super::ast_pipeline::composer_dep_extractor::extract_composer_dependencies),
        ("Gemfile", super::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies),
        ("C#", super::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies),
        ("SBT", super::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies),
        ("Swift", super::ast_pipeline::swift_dep_extractor::extract_swift_dependencies),
        ("Pubspec", super::ast_pipeline::pubspec_dep_extractor::extract_pubspec_dependencies),
    ];

    for (name, extractor) in extractors {
        match extractor(project_root) {
            Ok(dep_entities) => all_entities.extend(dep_entities),
            Err(e) => {
                tracing::warn!("{name} dependency extraction failed (non-fatal): {e}");
            }
        }
    }
}
