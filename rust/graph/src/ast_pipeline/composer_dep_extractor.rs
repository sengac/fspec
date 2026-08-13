//! Composer (PHP) Dependency Extractor
//!
//! Parses composer.json files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use serde_json::Value;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract PHP dependencies from composer.json.
pub fn extract_composer_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let composer_path = project_root.join("composer.json");
    if !composer_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&composer_path)
        .map_err(|e| format!("Failed to read composer.json: {e}"))?;

    let pkg: Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid composer.json: {e}"))?;

    let rel_path = "composer.json";
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "json", line_count, false,
    ));

    // Extract require
    if let Some(deps) = pkg.get("require").and_then(|v| v.as_object()) {
        for (name, version_val) in deps {
            let version = version_val.as_str().unwrap_or("*");
            entities.push(helpers::build_dependency_node(
                name, version, false, "composer",
            ));
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    // Extract require-dev
    if let Some(dev_deps) = pkg.get("require-dev").and_then(|v| v.as_object()) {
        for (name, version_val) in dev_deps {
            let version = version_val.as_str().unwrap_or("*");
            entities.push(helpers::build_dependency_node(
                name, version, true, "composer",
            ));
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    Ok(entities)
}
