//! Maven/Gradle (Java) Dependency Extractor
//!
//! Parses pom.xml and build.gradle files to extract Dependency nodes
//! for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Java dependencies from pom.xml and/or build.gradle.
pub fn extract_java_dependencies(project_root: &Path) -> helpers::Extracted {
    let mut extracted = helpers::Extracted::default();
    let gradle_path = project_root.join("build.gradle");
    if gradle_path.exists() {
        extracted.add(extract_build_gradle(project_root, &gradle_path));
    }
    let pom_path = project_root.join("pom.xml");
    if pom_path.exists() {
        extracted.add(extract_pom_xml(project_root, &pom_path));
    }
    extracted
}

/// The entities of build.gradle, or why it could not be read.
fn extract_build_gradle(
    project_root: &Path,
    gradle_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let mut entities = Vec::new();
    let content = helpers::read_manifest(project_root, gradle_path)?;

    let rel_path = "build.gradle";
    let file_slug = helpers::file_slug(rel_path);
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "groovy", line_count, false,
    ));

    for line in content.lines() {
        let trimmed = line.trim();
        // Lines like: implementation 'group:artifact:version'
        // or: testImplementation "group:artifact:version"
        if let Some((name, version, is_dev)) = parse_gradle_dependency(trimmed) {
            entities.push(helpers::build_dependency_node(
                &name, &version, is_dev, "gradle",
            ));
            entities.push(helpers::build_depends_on_edge(&file_slug, &name));
        }
    }

    Ok(entities)
}

/// The entities of pom.xml (basic text parsing, not full XML), or why it
/// could not be read.
fn extract_pom_xml(
    project_root: &Path,
    pom_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let mut entities = Vec::new();
    let content = helpers::read_manifest(project_root, pom_path)?;

    let rel_path = "pom.xml";
    let file_slug = helpers::file_slug(rel_path);
    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "xml", line_count, false,
    ));

    // Simple tag-based extraction
    let mut in_dependency = false;
    let mut artifact_id = String::new();
    let mut version = String::new();
    let mut scope = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.contains("<dependency>") {
            in_dependency = true;
            artifact_id.clear();
            version.clear();
            scope.clear();
        } else if trimmed.contains("</dependency>") && in_dependency {
            in_dependency = false;
            if !artifact_id.is_empty() {
                let is_dev = scope == "test";
                let ver = if version.is_empty() {
                    "*".to_string()
                } else {
                    version.clone()
                };
                entities.push(helpers::build_dependency_node(
                    &artifact_id,
                    &ver,
                    is_dev,
                    "maven",
                ));
                entities.push(helpers::build_depends_on_edge(&file_slug, &artifact_id));
            }
        } else if in_dependency {
            if let Some(val) = extract_xml_tag_value(trimmed, "artifactId") {
                artifact_id = val;
            }
            if let Some(val) = extract_xml_tag_value(trimmed, "version") {
                version = val;
            }
            if let Some(val) = extract_xml_tag_value(trimmed, "scope") {
                scope = val;
            }
        }
    }

    Ok(entities)
}

/// Parse a Gradle dependency line.
fn parse_gradle_dependency(line: &str) -> Option<(String, String, bool)> {
    let is_dev = line.starts_with("testImplementation") || line.starts_with("testCompile");
    let is_dep = is_dev
        || line.starts_with("implementation")
        || line.starts_with("compile")
        || line.starts_with("api");

    if !is_dep {
        return None;
    }

    // Find quoted string
    let quote_char = if line.contains('\'') { '\'' } else { '"' };
    if let Some(start) = line.find(quote_char) {
        let rest = &line[start + 1..];
        if let Some(end) = rest.find(quote_char) {
            let dep_str = &rest[..end];
            let parts: Vec<&str> = dep_str.split(':').collect();
            if parts.len() >= 2 {
                let name = parts[1];
                let version = if parts.len() >= 3 { parts[2] } else { "*" };
                return Some((name.to_string(), version.to_string(), is_dev));
            }
        }
    }
    None
}

/// Extract value between XML tags like `<tag>value</tag>`.
fn extract_xml_tag_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = line.find(&open) {
        let after = &line[start + open.len()..];
        if let Some(end) = after.find(&close) {
            return Some(after[..end].to_string());
        }
    }
    None
}
