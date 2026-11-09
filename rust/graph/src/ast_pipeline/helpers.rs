//! Entity builders shared by the dependency extractors.

use std::path::Path;

use serde_json::{Map, Value};

use crate::graph_entities::GraphEntity;

/// A File node's key: the path relative to the root with `/` separators
/// (KGRAPH-074: distinct paths never share a key).
pub fn file_slug(rel_path: &str) -> String {
    rel_path.replace('\\', "/")
}

/// Build a File node with standard properties.
pub fn build_file_node(
    rel_path: &str,
    file_slug: &str,
    language: &str,
    line_count: i32,
    is_test: bool,
) -> GraphEntity {
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(file_slug.to_string()));
    props.insert("path".to_string(), Value::String(rel_path.to_string()));
    props.insert("language".to_string(), Value::String(language.to_string()));
    props.insert("lineCount".to_string(), Value::Number(line_count.into()));
    props.insert("isTest".to_string(), Value::Bool(is_test));
    GraphEntity::Node {
        node_type: "File".to_string(),
        slug: file_slug.to_string(),
        properties: props,
    }
}

/// Build a Dependency node with standard properties.
///
/// Slug format: `dep::<package-name>` for upsert semantics.
pub fn build_dependency_node(name: &str, version: &str, is_dev: bool, source: &str) -> GraphEntity {
    let dep_slug = format!("dep::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(dep_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("version".to_string(), Value::String(version.to_string()));
    props.insert("isDev".to_string(), Value::Bool(is_dev));
    props.insert("source".to_string(), Value::String(source.to_string()));
    GraphEntity::Node {
        node_type: "Dependency".to_string(),
        slug: dep_slug,
        properties: props,
    }
}

/// Build a DependsOn edge from a File to a Dependency.
pub fn build_depends_on_edge(file_slug: &str, dep_name: &str) -> GraphEntity {
    let dep_slug = format!("dep::{dep_name}");
    GraphEntity::Edge {
        edge_type: "DependsOn".to_string(),
        from_slug: file_slug.to_string(),
        to_slug: dep_slug,
        properties: Map::new(),
    }
}

/// A package manifest a dependency extractor could not read or parse
/// (BUG-681): its path relative to the root and the exact error.
#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct ManifestError {
    pub file: String,
    pub error: String,
}

impl std::fmt::Display for ManifestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.file, self.error)
    }
}

impl std::error::Error for ManifestError {}

/// What a dependency extractor found: the entities of every manifest it
/// read, and every manifest it could not read or parse. One failed manifest
/// never stops the extractor's other manifests.
#[derive(Debug, Default)]
pub struct Extracted {
    pub entities: Vec<GraphEntity>,
    pub failures: Vec<ManifestError>,
}

impl Extracted {
    /// Add one manifest's entities, or its failure.
    pub fn add(&mut self, manifest: Result<Vec<GraphEntity>, ManifestError>) {
        match manifest {
            Ok(entities) => self.entities.extend(entities),
            Err(failure) => self.failures.push(failure),
        }
    }

    /// The result of an extractor that reads at most one manifest.
    pub fn of(manifest: Result<Vec<GraphEntity>, ManifestError>) -> Self {
        let mut extracted = Self::default();
        extracted.add(manifest);
        extracted
    }

    /// The entities when every manifest was read, else the first failure.
    pub fn into_entities(self) -> Result<Vec<GraphEntity>, ManifestError> {
        match self.failures.into_iter().next() {
            Some(failure) => Err(failure),
            None => Ok(self.entities),
        }
    }
}

/// The path of `path` relative to `root`, with `/` separators.
pub fn manifest_rel_path(root: &Path, path: &Path) -> String {
    file_slug(&path.strip_prefix(root).unwrap_or(path).to_string_lossy())
}

/// Read the manifest at `path`; a read failure names the manifest.
pub fn read_manifest(root: &Path, path: &Path) -> Result<String, ManifestError> {
    std::fs::read_to_string(path).map_err(|e| ManifestError {
        file: manifest_rel_path(root, path),
        error: format!("read failed: {e}"),
    })
}

/// Parse a JSON manifest read from `path`.
pub fn parse_json_manifest(
    root: &Path,
    path: &Path,
    content: &str,
) -> Result<serde_json::Value, ManifestError> {
    serde_json::from_str(content).map_err(|e| ManifestError {
        file: manifest_rel_path(root, path),
        error: format!("invalid JSON: {e}"),
    })
}

/// Parse a TOML manifest read from `path`.
pub fn parse_toml_manifest(
    root: &Path,
    path: &Path,
    content: &str,
) -> Result<toml::Value, ManifestError> {
    content.parse().map_err(|e| ManifestError {
        file: manifest_rel_path(root, path),
        error: format!("invalid TOML: {e}"),
    })
}
