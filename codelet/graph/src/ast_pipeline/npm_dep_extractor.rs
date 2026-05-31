//! NPM Dependency Extractor
//!
//! Parses package.json files to extract Dependency nodes and DependsOn edges
//! for the AST Connection Graph.

use std::path::Path;

use serde_json::Value;

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// Extract npm dependencies from a project's package.json.
///
/// Creates Dependency nodes for each dependency and devDependency,
/// plus DependsOn edges from the package.json File node.
pub fn extract_npm_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let pkg_path = project_root.join("package.json");
    if !pkg_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&pkg_path)
        .map_err(|e| format!("Failed to read package.json: {e}"))?;

    let pkg: Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid package.json: {e}"))?;

    let rel_path = "package.json";
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    // Create File node for package.json
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "json", line_count, false,
    ));

    // Extract dependencies
    if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        for (name, version_val) in deps {
            let version = version_val.as_str().unwrap_or("*");
            entities.push(helpers::build_dependency_node(name, version, false, "npm"));
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    // Extract devDependencies
    if let Some(dev_deps) = pkg.get("devDependencies").and_then(|v| v.as_object()) {
        for (name, version_val) in dev_deps {
            let version = version_val.as_str().unwrap_or("*");
            entities.push(helpers::build_dependency_node(name, version, true, "npm"));
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    Ok(entities)
}
