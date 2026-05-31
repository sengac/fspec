//! Dart/Flutter pubspec.yaml Dependency Extractor
//!
//! Parses `pubspec.yaml` to extract Dependency nodes for the AST Connection Graph.
//! Handles both `dependencies:` and `dev_dependencies:` sections.
//!
//! Uses simple line-based YAML parsing (no YAML library dependency) since
//! pubspec.yaml has a well-defined, flat structure for dependency declarations.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Dart/Flutter dependencies from pubspec.yaml.
///
/// Creates Dependency nodes for each package in `dependencies:` and
/// `dev_dependencies:` sections, plus DependsOn edges from the
/// pubspec.yaml File node.
pub fn extract_pubspec_dependencies(
    project_root: &Path,
) -> Result<Vec<GraphEntity>, String> {
    let pubspec_path = project_root.join("pubspec.yaml");
    if !pubspec_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&pubspec_path)
        .map_err(|e| format!("Failed to read pubspec.yaml: {e}"))?;

    let rel_path = "pubspec.yaml";
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "yaml", line_count, false,
    ));

    parse_pubspec_dependencies(&content, &file_slug, &mut entities);

    Ok(entities)
}

/// Parse dependency sections from pubspec.yaml content.
///
/// Recognises three sections:
/// - `dependencies:` → isDev = false
/// - `dev_dependencies:` → isDev = true
/// - `dependency_overrides:` → skipped
///
/// Each dependency is either a simple version constraint:
///   `provider: ^6.0.0`
/// or a complex map (sdk, git, path):
///   `flutter:\n    sdk: flutter`
///
/// We extract the package name and version (if available).
fn parse_pubspec_dependencies(
    content: &str,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    #[derive(PartialEq)]
    enum Section {
        None,
        Dependencies,
        DevDependencies,
        Other,
    }

    let mut section = Section::None;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Detect top-level section changes (no leading whitespace)
        if !line.starts_with(' ') && !line.starts_with('\t') {
            if trimmed == "dependencies:" {
                section = Section::Dependencies;
                continue;
            } else if trimmed == "dev_dependencies:" {
                section = Section::DevDependencies;
                continue;
            } else if trimmed.ends_with(':') {
                section = if trimmed == "dependency_overrides:" {
                    Section::Other
                } else {
                    Section::None
                };
                continue;
            }
        }

        // Only process lines within dependency sections
        if section != Section::Dependencies && section != Section::DevDependencies {
            continue;
        }

        let is_dev = section == Section::DevDependencies;

        // Check indentation level — direct dependencies are indented exactly 2 spaces
        if line.starts_with("  ") && !line.starts_with("    ") {
            let dep_line = trimmed;

            // Parse `package_name: version_constraint` or `package_name:`
            if let Some(colon_pos) = dep_line.find(':') {
                let name = dep_line[..colon_pos].trim();
                if name.is_empty() {
                    continue;
                }

                let version = dep_line[colon_pos + 1..].trim();
                let version = if version.is_empty() || version.starts_with('{') {
                    "*".to_string()
                } else {
                    version.to_string()
                };

                entities.push(helpers::build_dependency_node(
                    name, &version, is_dev, "pubspec",
                ));
                entities.push(helpers::build_depends_on_edge(file_slug, name));
            }
        }
    }
}
