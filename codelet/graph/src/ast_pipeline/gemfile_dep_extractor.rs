//! Gemfile (Ruby) Dependency Extractor
//!
//! Parses Gemfile files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Ruby dependencies from Gemfile.
pub fn extract_gemfile_dependencies(project_root: &Path) -> Result<Vec<GraphEntity>, String> {
    let gemfile_path = project_root.join("Gemfile");
    if !gemfile_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&gemfile_path)
        .map_err(|e| format!("Failed to read Gemfile: {e}"))?;

    let rel_path = "Gemfile";
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "ruby", line_count, false,
    ));

    let mut in_dev_group = false;
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("group :development") || trimmed.starts_with("group :test") {
            in_dev_group = true;
            continue;
        }
        if trimmed == "end" {
            in_dev_group = false;
            continue;
        }

        if trimmed.starts_with("gem ") || trimmed.starts_with("gem(") {
            if let Some((name, version)) = parse_gem_line(trimmed) {
                entities.push(helpers::build_dependency_node(
                    &name,
                    &version,
                    in_dev_group,
                    "gem",
                ));
                entities.push(helpers::build_depends_on_edge(&file_slug, &name));
            }
        }
    }

    Ok(entities)
}

/// Parse a gem line like `gem 'rails', '~> 7.0'` into (name, version).
fn parse_gem_line(line: &str) -> Option<(String, String)> {
    // Find the gem name in quotes
    let name = extract_quoted_string(line)?;
    
    // Try to find version after the name
    let after_name = &line[line.find(&name)? + name.len() + 1..];
    let version = extract_quoted_string(after_name).unwrap_or_else(|| "*".to_string());

    Some((name, version))
}

/// Extract the first quoted string from text.
fn extract_quoted_string(text: &str) -> Option<String> {
    for quote in &['\'', '"'] {
        if let Some(start) = text.find(*quote) {
            let rest = &text[start + 1..];
            if let Some(end) = rest.find(*quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}
