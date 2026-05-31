//! Incremental Re-indexing Helpers
//!
//! Functions for mtime-based incremental AST re-indexing. Instead of
//! re-extracting every file on each `ast_index` call, we:
//!
//! 1. Store file modification times on `File` graph nodes (`lastModified`).
//! 2. On subsequent index, compare current filesystem mtimes with stored ones.
//! 3. Only re-extract files whose mtime changed (or that are new).
//! 4. Reuse graph entities for unchanged files via `export_all_entities`.
//! 5. Combine fresh + reused entities and overwrite-load.
//!
//! Deleted files are handled implicitly — their entities are not included
//! in the reused set and therefore disappear on overwrite-load.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::database::GraphDatabase;
use crate::graph_entities::GraphEntity;

/// Collect filesystem modification times for a set of source files.
///
/// Returns a map of `relative_path → mtime_epoch_millis`.
/// Paths are relative to `project_root` with forward-slash separators.
pub fn collect_file_mtimes(
    files: &[std::path::PathBuf],
    project_root: &Path,
) -> HashMap<String, i64> {
    let mut mtimes = HashMap::new();
    for file in files {
        let rel = file
            .strip_prefix(project_root)
            .unwrap_or(file)
            .to_string_lossy()
            .replace('\\', "/");

        if let Ok(metadata) = std::fs::metadata(file) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    mtimes.insert(rel, duration.as_millis() as i64);
                }
            }
        }
    }
    mtimes
}

/// Read stored file modification times from the graph.
///
/// Exports all entities and extracts `lastModified` from File nodes.
/// Returns a map of `relative_path → mtime_epoch_millis`.
pub fn read_stored_mtimes(db: &GraphDatabase) -> Result<HashMap<String, i64>, String> {
    let entities = db.export_all_entities()?;
    let mut mtimes = HashMap::new();

    for entity in &entities {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "File" {
                let path = properties
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if path.is_empty() {
                    continue;
                }

                // lastModified is stored as ISO 8601 string by nanograph
                if let Some(last_modified) = properties.get("lastModified") {
                    if let Some(mtime_str) = last_modified.as_str() {
                        if let Ok(dt) =
                            chrono::NaiveDateTime::parse_from_str(mtime_str, "%Y-%m-%dT%H:%M:%S%.3fZ")
                        {
                            mtimes.insert(
                                path.to_string(),
                                dt.and_utc().timestamp_millis(),
                            );
                        } else if let Ok(dt) =
                            chrono::DateTime::parse_from_rfc3339(mtime_str)
                        {
                            mtimes.insert(
                                path.to_string(),
                                dt.timestamp_millis(),
                            );
                        }
                    } else if let Some(mtime_num) = last_modified.as_i64() {
                        mtimes.insert(path.to_string(), mtime_num);
                    }
                }
            }
        }
    }

    Ok(mtimes)
}

/// Partition files into changed, new, and deleted based on mtime comparison.
///
/// - **changed**: files that exist in both current and stored but with different mtimes
/// - **new**: files that exist in current but not in stored
/// - **deleted**: files that exist in stored but not in current
pub fn partition_changed_files(
    current_mtimes: &HashMap<String, i64>,
    stored_mtimes: &HashMap<String, i64>,
) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut changed = Vec::new();
    let mut new_files = Vec::new();
    let mut deleted = Vec::new();

    for (path, current_mtime) in current_mtimes {
        match stored_mtimes.get(path) {
            Some(stored_mtime) if stored_mtime != current_mtime => {
                changed.push(path.clone());
            }
            None => {
                new_files.push(path.clone());
            }
            _ => {} // unchanged
        }
    }

    for path in stored_mtimes.keys() {
        if !current_mtimes.contains_key(path) {
            deleted.push(path.clone());
        }
    }

    (changed, new_files, deleted)
}

/// Filter entities to keep only those NOT belonging to changed/deleted files.
///
/// Entity ownership is determined by slug prefix:
/// - File nodes: slug == file_slug
/// - Function/Type/Variable nodes: slug starts with `file_slug::`
/// - Edges: from_slug matches a changed file slug or starts with `file_slug::`
/// - Dependency nodes (slug starts with `dep::`) are always excluded
///   (they are re-extracted every time)
pub fn filter_reusable_entities(
    entities: Vec<GraphEntity>,
    changed_file_slugs: &HashSet<String>,
) -> Vec<GraphEntity> {
    entities
        .into_iter()
        .filter(|entity| match entity {
            GraphEntity::Node {
                node_type, slug, ..
            } => {
                // Always exclude Dependency nodes — re-extracted every time
                if node_type == "Dependency" {
                    return false;
                }
                // Check if this node belongs to a changed file
                !is_owned_by_changed_file(slug, changed_file_slugs)
            }
            GraphEntity::Edge {
                from_slug,
                edge_type,
                ..
            } => {
                // DependsOn edges are re-extracted with dependencies
                if edge_type == "DependsOn" {
                    return false;
                }
                // Edge is owned by the file that its from_slug belongs to
                !is_owned_by_changed_file(from_slug, changed_file_slugs)
            }
        })
        .collect()
}

/// Check if a slug belongs to one of the changed files.
///
/// A slug belongs to a file if:
/// - It equals the file slug (File node itself)
/// - It starts with `file_slug::` (Function, Type, Variable contained in the file)
fn is_owned_by_changed_file(slug: &str, changed_file_slugs: &HashSet<String>) -> bool {
    // Direct match (File node)
    if changed_file_slugs.contains(slug) {
        return true;
    }
    // Prefix match (Function::, Type::, Variable::)
    for file_slug in changed_file_slugs {
        let prefix = format!("{file_slug}::");
        if slug.starts_with(&prefix) {
            return true;
        }
    }
    false
}

/// Stamp `lastModified` on File nodes in an entity list.
///
/// Post-processes extracted entities: finds File nodes and sets their
/// `lastModified` property from the provided mtime map.
/// This avoids changing all 14 language extractor signatures.
pub fn stamp_file_mtimes(
    entities: &mut [GraphEntity],
    mtimes: &HashMap<String, i64>,
) {
    for entity in entities.iter_mut() {
        if let GraphEntity::Node {
            node_type,
            properties,
            ..
        } = entity
        {
            if node_type == "File" {
                let path = properties
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if let Some(&mtime_ms) = mtimes.get(&path) {
                    // Store as ISO 8601 string for nanograph DateTime compatibility
                    if let Some(dt) = chrono::DateTime::from_timestamp_millis(mtime_ms) {
                        properties.insert(
                            "lastModified".to_string(),
                            serde_json::Value::String(
                                dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                            ),
                        );
                    }
                }
            }
        }
    }
}
