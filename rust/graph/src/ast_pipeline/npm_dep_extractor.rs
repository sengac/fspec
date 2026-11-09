//! NPM Dependency Extractor
//!
//! Parses package.json files to extract Dependency nodes and DependsOn edges
//! for the AST Connection Graph.

use std::path::Path;

use serde_json::Value;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract npm dependencies from a project's package.json.
///
/// Creates Dependency nodes for each dependency and devDependency,
/// plus DependsOn edges from the package.json File node.
pub fn extract_npm_dependencies(project_root: &Path) -> helpers::Extracted {
    let pkg_path = project_root.join("package.json");
    if !pkg_path.exists() {
        return helpers::Extracted::default();
    }
    helpers::Extracted::of(extract_package_json(project_root, &pkg_path))
}

/// The entities of one package.json, or why it could not be read or parsed.
fn extract_package_json(
    project_root: &Path,
    pkg_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let content = helpers::read_manifest(project_root, pkg_path)?;
    let pkg: Value = helpers::parse_json_manifest(project_root, pkg_path, &content)?;

    let rel_path = "package.json";
    let file_slug = helpers::file_slug(rel_path);
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
