#![allow(clippy::await_holding_lock, clippy::type_complexity)]
//! AST Index Dispatch
//!
//! Handles the `ast_index` action: walks the project directory, extracts
//! AST entities (functions, types, imports, dependencies), and batch-loads
//! them into the AST code graph.
//!
//! Supports two modes:
//! - **Full** (default): re-extracts all files and overwrites the graph.
//! - **Incremental**: compares file mtimes, re-extracts only changed/new
//!   files, reuses unchanged entities from the graph, and overwrites.
//!
//! Extracted from `ast_dispatch.rs` for file size compliance.

use std::collections::HashSet;

use crate::ast_pipeline::incremental::{
    collect_file_mtimes, filter_reusable_entities, partition_changed_files, read_stored_mtimes,
    stamp_file_mtimes,
};
use crate::dispatch_helpers::format_graph_stats;

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
///
/// When `incremental` is `true`, compares file modification times against
/// stored values and only re-extracts files that have changed. Falls back
/// to full extraction when no prior index exists or >50% of files changed.
pub async fn dispatch_ast_index(
    custom_path: Option<&str>,
    reset: bool,
    incremental: bool,
) -> String {
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

    // ── Incremental path ──────────────────────────────────────
    if incremental {
        return dispatch_incremental(&project_root, respect_gitignore, &db).await;
    }

    // ── Full path (default) ───────────────────────────────────
    dispatch_full(&project_root, respect_gitignore, &db).await
}

/// Full extraction: walk + extract all + stamp mtimes + overwrite.
async fn dispatch_full(
    project_root: &std::path::Path,
    respect_gitignore: bool,
    db: &super::database::GraphDatabase,
) -> String {
    // Walk codebase and extract AST entities
    let mut all_entities =
        match super::ast_pipeline::walk_and_extract(project_root, respect_gitignore) {
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
    extract_all_dependencies(project_root, &mut all_entities);

    // Deduplicate after merging dep-extractor results with AST entities.
    let mut all_entities = super::ast_pipeline::deduplicate_entities(all_entities);

    // Stamp file mtimes for incremental support
    let source_files = super::ast_pipeline::walk_source_files(project_root, respect_gitignore);
    let mtimes = collect_file_mtimes(&source_files, project_root);
    stamp_file_mtimes(&mut all_entities, &mtimes);

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
            build_index_result(db, loaded, 0, false)
        }
        Err(e) => serde_json::json!({
            "action": "ast_index",
            "error": format!("Failed to load entities into graph: {e}"),
        })
        .to_string(),
    }
}

/// Incremental extraction: compare mtimes, extract only changed files,
/// reuse unchanged entities, combine and overwrite.
async fn dispatch_incremental(
    project_root: &std::path::Path,
    respect_gitignore: bool,
    db: &super::database::GraphDatabase,
) -> String {
    // Phase 1: Walk filesystem to collect all current source files + mtimes
    let source_files = super::ast_pipeline::walk_source_files(project_root, respect_gitignore);
    let current_mtimes = collect_file_mtimes(&source_files, project_root);

    if current_mtimes.is_empty() {
        return serde_json::json!({
            "action": "ast_index",
            "incremental": true,
            "entities_loaded": 0,
            "message": "No source files found to index",
        })
        .to_string();
    }

    // Phase 2: Read stored mtimes from graph
    let stored_mtimes = match read_stored_mtimes(db) {
        Ok(m) => m,
        Err(e) => {
            tracing::warn!("Failed to read stored mtimes, falling back to full: {e}");
            return dispatch_full(project_root, respect_gitignore, db).await;
        }
    };

    // Phase 3: Partition files
    let (changed, new_files, deleted) = partition_changed_files(&current_mtimes, &stored_mtimes);

    let total_needing_extraction = changed.len() + new_files.len();

    // Phase 4: Decide strategy — fall back to full if no prior index or >50% changed
    if stored_mtimes.is_empty() || total_needing_extraction * 2 > current_mtimes.len() {
        tracing::info!(
            stored = stored_mtimes.len(),
            changed = changed.len(),
            new = new_files.len(),
            total = current_mtimes.len(),
            "Incremental: falling back to full extraction"
        );
        return dispatch_full(project_root, respect_gitignore, db).await;
    }

    // Phase 5: If nothing changed, return early
    if total_needing_extraction == 0 && deleted.is_empty() {
        tracing::info!("Incremental: no changes detected, skipping re-index");
        return build_index_result(db, 0, 0, true);
    }

    tracing::info!(
        changed = changed.len(),
        new = new_files.len(),
        deleted = deleted.len(),
        unchanged = current_mtimes.len() - total_needing_extraction,
        "Incremental: selective re-extraction"
    );

    // Phase 6: Build set of file slugs that need re-extraction
    let changed_slugs: HashSet<String> = changed
        .iter()
        .chain(new_files.iter())
        .chain(deleted.iter())
        .map(|p| super::ast_pipeline::helpers::slugify_path(p))
        .collect();

    // Phase 7: Export unchanged entities from graph
    let existing = match db.export_all_entities() {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to export entities, falling back to full: {e}");
            return dispatch_full(project_root, respect_gitignore, db).await;
        }
    };
    let reused = filter_reusable_entities(existing, &changed_slugs);

    // Phase 8: Extract fresh entities for changed/new files only
    let files_to_extract: Vec<std::path::PathBuf> = changed
        .iter()
        .chain(new_files.iter())
        .filter_map(|rel| {
            let full = project_root.join(rel);
            if full.exists() {
                Some(full)
            } else {
                None
            }
        })
        .collect();

    // Build known_files from ALL current files (needed for barrel-import resolution)
    let known_files: HashSet<String> = source_files
        .iter()
        .filter_map(|p| {
            p.strip_prefix(project_root)
                .ok()
                .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        })
        .collect();

    let mut fresh_entities = Vec::new();
    for path in &files_to_extract {
        match super::ast_pipeline::extract_file(path, project_root, &known_files) {
            Ok(entities) => fresh_entities.extend(entities),
            Err(e) => {
                tracing::warn!(?path, error = %e, "incremental: failed to extract AST");
            }
        }
    }

    // Stamp mtimes on fresh File nodes
    let fresh_mtimes: std::collections::HashMap<String, i64> = changed
        .iter()
        .chain(new_files.iter())
        .filter_map(|rel| current_mtimes.get(rel).map(|m| (rel.clone(), *m)))
        .collect();
    stamp_file_mtimes(&mut fresh_entities, &fresh_mtimes);

    // Phase 9: Re-extract dependencies (always — fast and small)
    extract_all_dependencies(project_root, &mut fresh_entities);

    // Phase 10: Combine reused + fresh, deduplicate, overwrite
    let mut combined = reused;
    combined.extend(fresh_entities);
    let combined = super::ast_pipeline::deduplicate_entities(combined);
    let re_extracted = files_to_extract.len();

    match db.load_entities_overwrite(&combined).await {
        Ok(loaded) => {
            tracing::info!(loaded, re_extracted, "Incremental AST index complete");
            build_index_result(db, loaded, re_extracted, true)
        }
        Err(e) => serde_json::json!({
            "action": "ast_index",
            "incremental": true,
            "error": format!("Failed to load entities into graph: {e}"),
        })
        .to_string(),
    }
}

/// Build the JSON result string with graph stats.
fn build_index_result(
    db: &super::database::GraphDatabase,
    loaded: usize,
    re_extracted: usize,
    incremental: bool,
) -> String {
    let stats = format_graph_stats(db, "ast_index");
    if let Ok(mut parsed) = serde_json::from_str::<serde_json::Value>(&stats) {
        if let Some(obj) = parsed.as_object_mut() {
            obj.insert(
                "entities_loaded".to_string(),
                serde_json::Value::Number(loaded.into()),
            );
            if incremental {
                obj.insert("incremental".to_string(), serde_json::Value::Bool(true));
                obj.insert(
                    "files_re_extracted".to_string(),
                    serde_json::Value::Number(re_extracted.into()),
                );
            }
        }
        parsed.to_string()
    } else {
        stats
    }
}

/// Extract dependencies from all supported package managers into the entity list.
fn extract_all_dependencies(
    project_root: &std::path::Path,
    all_entities: &mut Vec<super::graph_entities::GraphEntity>,
) {
    let extractors: &[(
        &str,
        fn(&std::path::Path) -> Result<Vec<super::graph_entities::GraphEntity>, String>,
    )] = &[
        (
            "Cargo",
            super::ast_pipeline::cargo_dep_extractor::extract_cargo_dependencies,
        ),
        (
            "NPM",
            super::ast_pipeline::npm_dep_extractor::extract_npm_dependencies,
        ),
        (
            "Python",
            super::ast_pipeline::pip_dep_extractor::extract_python_dependencies,
        ),
        (
            "Go",
            super::ast_pipeline::gomod_dep_extractor::extract_go_dependencies,
        ),
        (
            "Java",
            super::ast_pipeline::java_dep_extractor::extract_java_dependencies,
        ),
        (
            "Composer",
            super::ast_pipeline::composer_dep_extractor::extract_composer_dependencies,
        ),
        (
            "Gemfile",
            super::ast_pipeline::gemfile_dep_extractor::extract_gemfile_dependencies,
        ),
        (
            "C#",
            super::ast_pipeline::csproj_dep_extractor::extract_csproj_dependencies,
        ),
        (
            "SBT",
            super::ast_pipeline::sbt_dep_extractor::extract_sbt_dependencies,
        ),
        (
            "Swift",
            super::ast_pipeline::swift_dep_extractor::extract_swift_dependencies,
        ),
        (
            "Pubspec",
            super::ast_pipeline::pubspec_dep_extractor::extract_pubspec_dependencies,
        ),
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
