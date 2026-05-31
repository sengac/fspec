//! Go Module Dependency Extractor
//!
//! Parses go.mod files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Go dependencies from go.mod.
pub fn extract_go_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let gomod_path = project_root.join("go.mod");
    if !gomod_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&gomod_path)
        .map_err(|e| format!("Failed to read go.mod: {e}"))?;

    let rel_path = "go.mod";
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "go", line_count, false,
    ));

    let mut in_require = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "require (" {
            in_require = true;
            continue;
        }
        if trimmed == ")" {
            in_require = false;
            continue;
        }

        if in_require {
            // Lines like: github.com/pkg/errors v0.9.1
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1];
                let is_indirect = trimmed.contains("// indirect");
                entities.push(helpers::build_dependency_node(
                    name,
                    version,
                    is_indirect,
                    "go",
                ));
                entities.push(helpers::build_depends_on_edge(&file_slug, name));
            }
        } else if let Some(rest) = trimmed.strip_prefix("require ") {
            // Single-line require: require github.com/pkg/errors v0.9.1
            let parts: Vec<&str> = rest.split_whitespace().collect();
            if parts.len() >= 2 {
                let name = parts[0];
                let version = parts[1];
                entities.push(helpers::build_dependency_node(
                    name,
                    version,
                    false,
                    "go",
                ));
                entities.push(helpers::build_depends_on_edge(&file_slug, name));
            }
        }
    }

    Ok(entities)
}
