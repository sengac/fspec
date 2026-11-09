//! SBT (Scala) Dependency Extractor
//!
//! Parses build.sbt files to extract Dependency nodes for the AST Connection Graph.

use std::path::Path;

use super::helpers;
use crate::graph_entities::GraphEntity;

/// Extract Scala dependencies from build.sbt.
pub fn extract_sbt_dependencies(project_root: &Path) -> helpers::Extracted {
    let sbt_path = project_root.join("build.sbt");
    if !sbt_path.exists() {
        return helpers::Extracted::default();
    }
    helpers::Extracted::of(extract_build_sbt(project_root, &sbt_path))
}

/// The entities of one build.sbt, or why it could not be read.
fn extract_build_sbt(
    project_root: &Path,
    sbt_path: &Path,
) -> Result<Vec<GraphEntity>, helpers::ManifestError> {
    let content = helpers::read_manifest(project_root, sbt_path)?;

    let rel_path = "build.sbt";
    let file_slug = helpers::file_slug(rel_path);
    let mut entities = Vec::new();

    let line_count = content.lines().count() as i32;
    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "scala", line_count, false,
    ));

    // Look for quoted strings separated by % or %%
    // Pattern: "org" %% "artifact" % "version"
    // or: "org" % "artifact" % "version"
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((name, version, is_test)) = parse_sbt_dependency(trimmed) {
            entities.push(helpers::build_dependency_node(
                &name, &version, is_test, "sbt",
            ));
            entities.push(helpers::build_depends_on_edge(&file_slug, &name));
        }
    }

    Ok(entities)
}

/// Parse an SBT dependency line into (artifact_name, version, is_test).
///
/// Matches patterns like:
/// - `"com.typesafe.akka" %% "akka-actor" % "2.8.0"`
/// - `"org.scalatest" %% "scalatest" % "3.2.0" % Test`
fn parse_sbt_dependency(line: &str) -> Option<(String, String, bool)> {
    // Collect all quoted strings
    let mut quoted_strings = Vec::new();
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            let mut s = String::new();
            for inner in chars.by_ref() {
                if inner == '"' {
                    break;
                }
                s.push(inner);
            }
            if !s.is_empty() {
                quoted_strings.push(s);
            }
        }
    }

    // Need at least group and artifact (2 strings), version optional (3 strings)
    if quoted_strings.len() < 2 {
        return None;
    }

    // Must contain % separator
    if !line.contains('%') {
        return None;
    }

    let artifact = &quoted_strings[1];
    let version = if quoted_strings.len() >= 3 {
        quoted_strings[2].clone()
    } else {
        "*".to_string()
    };
    let is_test = line.contains("% Test") || line.contains("% test");

    Some((artifact.clone(), version, is_test))
}
