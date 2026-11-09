//! C# (.csproj) Dependency Extractor
//!
//! Parses .csproj files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract C# dependencies from .csproj files.
pub fn extract_csproj_dependencies(project_root: &Path) -> helpers::Extracted {
    let mut extracted = helpers::Extracted::default();
    // Find .csproj files in the root
    let root_unreadable = |e: std::io::Error| helpers::ManifestError {
        file: ".".to_string(),
        error: format!("read failed: {e}"),
    };
    let entries = match std::fs::read_dir(project_root) {
        Ok(entries) => entries,
        Err(e) => {
            extracted.failures.push(root_unreadable(e));
            return extracted;
        }
    };
    let mut paths = Vec::new();
    for entry in entries {
        match entry {
            Ok(entry) => paths.push(entry.path()),
            Err(e) => extracted.failures.push(root_unreadable(e)),
        }
    }
    paths.sort();
    for path in paths {
        if path.extension().and_then(|e| e.to_str()) == Some("csproj") {
            extracted.add(extract_csproj(project_root, &path));
        }
    }
    extracted
}

/// The entities of one .csproj file, or why it could not be read.
fn extract_csproj(
    project_root: &Path,
    path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let mut entities = Vec::new();
    let content = helpers::read_manifest(project_root, path)?;

    let rel_path = helpers::manifest_rel_path(project_root, path);
    let rel_path = rel_path.as_str();
    let file_slug = helpers::file_slug(rel_path);
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
