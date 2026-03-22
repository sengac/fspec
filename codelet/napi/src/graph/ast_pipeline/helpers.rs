//! Shared helpers for AST extractors.
//!
//! Common utility functions used by both TypeScript and Rust extractors
//! to build graph entities consistently.

use serde_json::{Map, Value};

use crate::graph::graph_entities::GraphEntity;

/// Slugify a relative file path for use as a graph node key.
///
/// Replaces path separators and dots with hyphens.
pub fn slugify_path(path: &str) -> String {
    path.replace(['/', '\\'], "-")
        .replace('.', "-")
        .trim_matches('-')
        .to_string()
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

/// Build a Function node with standard properties.
pub fn build_function_node(
    file_slug: &str,
    name: &str,
    is_async: bool,
    is_public: bool,
    param_count: i32,
    line_start: i32,
    line_end: i32,
) -> GraphEntity {
    let fn_slug = format!("{file_slug}::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(fn_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("qualifiedName".to_string(), Value::String(fn_slug.clone()));
    props.insert("isAsync".to_string(), Value::Bool(is_async));
    props.insert("isPublic".to_string(), Value::Bool(is_public));
    props.insert("paramCount".to_string(), Value::Number(param_count.into()));
    props.insert("lineStart".to_string(), Value::Number(line_start.into()));
    props.insert("lineEnd".to_string(), Value::Number(line_end.into()));
    GraphEntity::Node {
        node_type: "Function".to_string(),
        slug: fn_slug,
        properties: props,
    }
}

/// Build a Contains edge from a File to a child entity.
pub fn build_contains_edge(
    file_slug: &str,
    child_slug: &str,
    edge_type: &str,
) -> GraphEntity {
    GraphEntity::Edge {
        edge_type: edge_type.to_string(),
        from_slug: file_slug.to_string(),
        to_slug: child_slug.to_string(),
        properties: Map::new(),
    }
}

/// Build a Type node with standard properties.
pub fn build_type_node(
    file_slug: &str,
    name: &str,
    type_kind: &str,
    is_public: bool,
) -> GraphEntity {
    let type_slug = format!("{file_slug}::{name}");
    let mut props = Map::new();
    props.insert("slug".to_string(), Value::String(type_slug.clone()));
    props.insert("name".to_string(), Value::String(name.to_string()));
    props.insert("typeKind".to_string(), Value::String(type_kind.to_string()));
    props.insert("isPublic".to_string(), Value::Bool(is_public));
    GraphEntity::Node {
        node_type: "Type".to_string(),
        slug: type_slug,
        properties: props,
    }
}

/// Count parameters by splitting on commas between the first `(` and `)`.
///
/// Handles both TypeScript and Rust parameter lists.
pub fn count_params(text: &str) -> i32 {
    if let Some(open) = text.find('(') {
        if let Some(close) = text.find(')') {
            let params = text[open + 1..close].trim();
            if params.is_empty() {
                return 0;
            }
            return params.matches(',').count() as i32 + 1;
        }
    }
    0
}

/// Count Rust parameters, filtering out `self`, `&self`, `&mut self`.
pub fn count_params_rust(text: &str) -> i32 {
    if let Some(open) = text.find('(') {
        if let Some(close) = text.find(')') {
            let params = text[open + 1..close].trim();
            if params.is_empty() {
                return 0;
            }
            let params_str = params
                .replace("&self", "")
                .replace("&mut self", "")
                .replace("self", "");
            let trimmed = params_str.trim().trim_matches(',').trim();
            if trimmed.is_empty() {
                return 0;
            }
            return trimmed.matches(',').count() as i32 + 1;
        }
    }
    0
}

/// Build a Dependency node with standard properties.
///
/// Slug format: `dep::<package-name>` for upsert semantics.
pub fn build_dependency_node(
    name: &str,
    version: &str,
    is_dev: bool,
    source: &str,
) -> GraphEntity {
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

/// Extract an identifier name after a keyword (e.g., "function " → name, "fn " → name).
///
/// Shared by both TypeScript and Rust extractors to avoid duplicated name-parsing logic.
/// Returns an empty string if the keyword is not found.
pub fn extract_name_after_keyword(text: &str, keyword: &str) -> String {
    if let Some(pos) = text.find(keyword) {
        let after = &text[pos + keyword.len()..];
        return after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
    }
    String::new()
}
