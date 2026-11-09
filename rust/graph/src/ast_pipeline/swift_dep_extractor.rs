//! Swift Package Manager Dependency Extractor
//!
//! Parses Package.swift to extract Dependency nodes for the AST Connection Graph.
//! Uses simple text scanning since Package.swift is Swift source code.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Swift dependencies from Package.swift.
pub fn extract_swift_dependencies(project_root: &Path) -> helpers::Extracted {
    let pkg_path = project_root.join("Package.swift");
    if !pkg_path.exists() {
        return helpers::Extracted::default();
    }
    helpers::Extracted::of(extract_package_swift(project_root, &pkg_path))
}

/// The entities of one Package.swift, or why it could not be read.
fn extract_package_swift(
    project_root: &Path,
    pkg_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let content = helpers::read_manifest(project_root, pkg_path)?;

    let rel_path = "Package.swift";
    let file_slug = helpers::file_slug(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "swift", line_count, false,
    ));

    // Look for .package(url: "https://github.com/user/repo", ...) patterns
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains(".package(") && trimmed.contains("url:") {
            if let Some(url) = extract_swift_package_url(trimmed) {
                // Extract repo name from URL
                let name = url
                    .trim_end_matches(".git")
                    .rsplit('/')
                    .next()
                    .unwrap_or(&url);
                let version = extract_swift_package_version(trimmed);
                entities.push(helpers::build_dependency_node(name, &version, false, "spm"));
                entities.push(helpers::build_depends_on_edge(&file_slug, name));
            }
        }
    }

    Ok(entities)
}

/// Extract URL from a Swift package declaration line.
fn extract_swift_package_url(line: &str) -> Option<String> {
    if let Some(url_pos) = line.find("url:") {
        let after = &line[url_pos + 4..];
        let after = after.trim().trim_start_matches('"');
        if let Some(end) = after.find('"') {
            return Some(after[..end].to_string());
        }
    }
    None
}

/// Extract version from a Swift package declaration line.
fn extract_swift_package_version(line: &str) -> String {
    // Look for from: "x.y.z" or exact: "x.y.z"
    for keyword in &["from:", "exact:"] {
        if let Some(pos) = line.find(keyword) {
            let after = &line[pos + keyword.len()..];
            let after = after.trim().trim_start_matches('"');
            if let Some(end) = after.find('"') {
                return after[..end].to_string();
            }
        }
    }
    "*".to_string()
}
