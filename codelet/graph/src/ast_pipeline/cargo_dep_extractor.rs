//! Cargo Dependency Extractor
//!
//! Parses Cargo.toml files (including workspace manifests) to extract
//! Dependency nodes and DependsOn edges for the AST Connection Graph.

use std::collections::HashSet;
use std::path::Path;

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// Extract Cargo dependencies from a project root.
///
/// Handles both single-crate and workspace manifests.
/// For workspaces, scans all member crate Cargo.toml files.
pub fn extract_cargo_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let cargo_path = project_root.join("Cargo.toml");
    if !cargo_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&cargo_path)
        .map_err(|e| format!("Failed to read Cargo.toml: {e}"))?;

    let parsed: toml::Value =
        content.parse().map_err(|e| format!("Invalid Cargo.toml: {e}"))?;

    let mut entities = Vec::new();
    let mut seen_deps = HashSet::new();

    // Check if this is a workspace
    if let Some(workspace) = parsed.get("workspace") {
        // Extract workspace-level deps first
        extract_workspace_deps(workspace, &mut entities, &mut seen_deps);

        // Process each member crate
        if let Some(members) = workspace.get("members").and_then(|v| v.as_array()) {
            for member in members {
                if let Some(member_path) = member.as_str() {
                    let member_cargo = project_root.join(member_path).join("Cargo.toml");
                    if member_cargo.exists() {
                        extract_single_crate(
                            &member_cargo,
                            project_root,
                            &mut entities,
                            &mut seen_deps,
                        )?;
                    }
                }
            }
        }
    }

    // Always process the root Cargo.toml if it has [package]
    if parsed.get("package").is_some() {
        extract_single_crate(&cargo_path, project_root, &mut entities, &mut seen_deps)?;
    }

    Ok(entities)
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
    entities: &mut Vec<GraphEntity>,
    seen_deps: &mut HashSet<String>,
) -> Result<(), String> {
    let content = std::fs::read_to_string(cargo_path)
        .map_err(|e| format!("Failed to read {}: {e}", cargo_path.display()))?;

    let parsed: toml::Value = content
        .parse()
        .map_err(|e| format!("Invalid {}: {e}", cargo_path.display()))?;

    let rel_path = cargo_path
        .strip_prefix(project_root)
        .unwrap_or(cargo_path)
        .to_string_lossy()
        .to_string();

    let file_slug = helpers::slugify_path(&rel_path);

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

    Ok(())
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
