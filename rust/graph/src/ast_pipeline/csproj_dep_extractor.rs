//! C# (.csproj) Dependency Extractor
//!
//! Parses .csproj files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract C# dependencies from .csproj files.
pub fn extract_csproj_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    // Find .csproj files in the root
    let entries =
        std::fs::read_dir(project_root).map_err(|e| format!("Failed to read project root: {e}"))?;

    let mut entities = Vec::new();

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("csproj") {
            continue;
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

        let rel_path = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project.csproj");
        let file_slug = helpers::slugify_path(rel_path);
        let line_count = content.lines().count() as i32;

        entities.push(helpers::build_file_node(
            rel_path, &file_slug, "xml", line_count, false,
        ));

        // Parse <PackageReference Include="Name" Version="Version" />
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("PackageReference") && trimmed.contains("Include=") {
                if let Some((name, version)) = parse_package_reference(trimmed) {
                    entities.push(helpers::build_dependency_node(
                        &name, &version, false, "nuget",
                    ));
                    entities.push(helpers::build_depends_on_edge(&file_slug, &name));
                }
            }
        }
    }

    Ok(entities)
}

/// Parse a PackageReference XML element.
fn parse_package_reference(line: &str) -> Option<(String, String)> {
    let name = extract_xml_attribute(line, "Include")?;
    let version = extract_xml_attribute(line, "Version").unwrap_or_else(|| "*".to_string());
    Some((name, version))
}

/// Extract an XML attribute value like `Attr="value"`.
fn extract_xml_attribute(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    if let Some(start) = line.find(&pattern) {
        let after = &line[start + pattern.len()..];
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}
