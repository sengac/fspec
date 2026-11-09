//! Python Dependency Extractor
//!
//! Parses requirements.txt and pyproject.toml files to extract Dependency nodes
//! for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Python dependencies from a project root.
///
/// Checks requirements.txt first, then pyproject.toml.
pub fn extract_python_dependencies(project_root: &Path) -> helpers::Extracted {
    let mut extracted = helpers::Extracted::default();
    let req_path = project_root.join("requirements.txt");
    if req_path.exists() {
        extracted.add(extract_requirements_txt(project_root, &req_path));
    }
    let pyproject_path = project_root.join("pyproject.toml");
    if pyproject_path.exists() {
        extracted.add(extract_pyproject_toml(project_root, &pyproject_path));
    }
    extracted
}

/// The entities of requirements.txt, or why it could not be read.
fn extract_requirements_txt(
    project_root: &Path,
    req_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let mut entities = Vec::new();
    let content = helpers::read_manifest(project_root, req_path)?;

    let rel_path = "requirements.txt";
    let file_slug = helpers::file_slug(rel_path);
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "text", line_count, false,
    ));

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
            continue;
        }
        let (name, version) = parse_requirement(line);
        if !name.is_empty() {
            entities.push(helpers::build_dependency_node(
                &name, &version, false, "pip",
            ));
            entities.push(helpers::build_depends_on_edge(&file_slug, &name));
        }
    }

    Ok(entities)
}

/// The entities of pyproject.toml, or why it could not be read or parsed.
fn extract_pyproject_toml(
    project_root: &Path,
    pyproject_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let mut entities = Vec::new();
    let content = helpers::read_manifest(project_root, pyproject_path)?;
    let parsed = helpers::parse_toml_manifest(project_root, pyproject_path, &content)?;

    let rel_path = "pyproject.toml";
    let file_slug = helpers::file_slug(rel_path);
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "toml", line_count, false,
    ));

    // [project.dependencies]
    if let Some(deps) = parsed
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for dep in deps {
            if let Some(dep_str) = dep.as_str() {
                let (name, version) = parse_requirement(dep_str);
                if !name.is_empty() {
                    entities.push(helpers::build_dependency_node(
                        &name, &version, false, "pip",
                    ));
                    entities.push(helpers::build_depends_on_edge(&file_slug, &name));
                }
            }
        }
    }

    // [project.optional-dependencies] (treat as dev)
    if let Some(opt_deps) = parsed
        .get("project")
        .and_then(|p| p.get("optional-dependencies"))
        .and_then(|d| d.as_table())
    {
        for (_group, deps) in opt_deps {
            if let Some(arr) = deps.as_array() {
                for dep in arr {
                    if let Some(dep_str) = dep.as_str() {
                        let (name, version) = parse_requirement(dep_str);
                        if !name.is_empty() {
                            entities
                                .push(helpers::build_dependency_node(&name, &version, true, "pip"));
                            entities.push(helpers::build_depends_on_edge(&file_slug, &name));
                        }
                    }
                }
            }
        }
    }

    Ok(entities)
}

/// Parse a requirement line like "flask>=2.0" into (name, version).
fn parse_requirement(line: &str) -> (String, String) {
    // Handle extras: "package[extra]>=1.0"
    let line = if let Some(bracket_pos) = line.find('[') {
        if let Some(close) = line.find(']') {
            format!("{}{}", &line[..bracket_pos], &line[close + 1..])
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    };

    for sep in &[">=", "<=", "==", "!=", "~=", ">", "<"] {
        if let Some(pos) = line.find(sep) {
            let name = line[..pos].trim().to_lowercase();
            let version = line[pos..].trim().to_string();
            return (name, version);
        }
    }

    let name = line
        .split(';')
        .next()
        .unwrap_or(&line)
        .trim()
        .to_lowercase();
    (name, "*".to_string())
}
