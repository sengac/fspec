//! Cargo Dependency Extractor
//!
//! Parses Cargo.toml files (including workspace manifests) to extract
//! Dependency nodes and DependsOn edges for the AST Connection Graph.

use std::collections::HashSet;
use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Cargo dependencies from a project root.
///
/// Handles both single-crate and workspace manifests.
/// For workspaces, scans all member crate Cargo.toml files.
pub fn extract_cargo_dependencies(project_root: &Path) -> helpers::Extracted {
    let mut extracted = helpers::Extracted::default();
    let cargo_path = project_root.join("Cargo.toml");
    if !cargo_path.exists() {
        return extracted;
    }
    let parsed = match helpers::read_manifest(project_root, &cargo_path)
        .and_then(|content| helpers::parse_toml_manifest(project_root, &cargo_path, &content))
    {
        Ok(parsed) => parsed,
        Err(failure) => {
            extracted.failures.push(failure);
            return extracted;
        }
    };

    let mut seen_deps = HashSet::new();

    // Check if this is a workspace
    if let Some(workspace) = parsed.get("workspace") {
        // Extract workspace-level deps first
        extract_workspace_deps(workspace, &mut extracted.entities, &mut seen_deps);

        // Process each member crate; a member that fails is listed and the
        // other members are still read (BUG-681).
        if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
            for member in members {
                if let Some(member_path) = member.as_str() {
                    let member_cargo = project_root.join(member_path).join("Cargo.toml");
                    if member_cargo.exists() {
                        extracted.add(extract_single_crate(
                            &member_cargo,
                            project_root,
                            &mut seen_deps,
                        ));
                    }
                }
            }
        }
    }

    // Always process the root Cargo.toml if it has [package]
    if parsed.get("package").is_some() {
        extracted.add(extract_single_crate(
            &cargo_path,
            project_root,
            &mut seen_deps,
        ));
    }

    extracted
}

/// Extract dependencies from workspace-level `[workspace.dependencies]`.
fn extract_workspace_deps(
    workspace: &toml::Value,
    entities: &mut Vec<GraphEntity>,
    seen_deps: &mut HashSet<String>,
) {
    if let Some(deps) = workspace.get("dependencies").and_then(|v| v.as_table()) {
        for (name, value) in deps {
            if seen_deps.insert(name.clone()) {
                let version = extract_version(value);
                entities.push(helpers::build_dependency_node(
                    name, &version, false, "crate",
                ));
            }
        }
    }
}

/// Extract dependencies from a single crate's Cargo.toml.
fn extract_single_crate(
    cargo_path: &Path,
    project_root: &Path,
    seen_deps: &mut HashSet<String>,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let content = helpers::read_manifest(project_root, cargo_path)?;
    let parsed = helpers::parse_toml_manifest(project_root, cargo_path, &content)?;

    let rel_path = helpers::manifest_rel_path(project_root, cargo_path);
    let file_slug = helpers::file_slug(&rel_path);
    let mut entities = Vec::new();

    // Create File node for this Cargo.toml
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        &rel_path, &file_slug, "toml", line_count, false,
    ));

    // Extract [dependencies]
    if let Some(deps) = parsed.get("dependencies").and_then(|v| v.as_table()) {
        for (name, value) in deps {
            if seen_deps.insert(name.clone()) {
                let version = extract_version(value);
                entities.push(helpers::build_dependency_node(
                    name, &version, false, "crate",
                ));
            }
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    // Extract [dev-dependencies]
    if let Some(dev_deps) = parsed.get("dev-dependencies").and_then(|v| v.as_table()) {
        for (name, value) in dev_deps {
            if seen_deps.insert(name.clone()) {
                let version = extract_version(value);
                entities.push(helpers::build_dependency_node(
                    name, &version, true, "crate",
                ));
            }
            entities.push(helpers::build_depends_on_edge(&file_slug, name));
        }
    }

    Ok(entities)
}

/// Extract version string from a TOML dependency value.
///
/// Handles both simple string versions (`"1.0"`) and table forms
/// (`{ version = "1.0", features = [...] }`) and workspace references.
fn extract_version(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Table(t) => {
            if t.contains_key("workspace") {
                "workspace".to_string()
            } else if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                v.to_string()
            } else {
                "*".to_string()
            }
        }
        _ => "*".to_string(),
    }
}
