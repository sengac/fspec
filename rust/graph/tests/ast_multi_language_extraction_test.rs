// Feature: spec/features/multi-language-ast-extraction.feature
//
// Multi-Language AST Extraction Pipeline Tests
// Tests for extracting AST entities from Python, Go, Java, C, C++, C#,
// Ruby, Kotlin, Swift, Scala, and PHP source files.
//
// Each test indexes an isolated temp directory of synthetic source files
// with the fact engine and resolve pass (`index_tree`).

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::Path;

use codelet_graph::ast_pipeline::index_tree;
use codelet_graph::graph_entities::GraphEntity;
use serde_json::Value;

/// Write a file and return its path.
fn write_test_file(dir: &Path, rel_path: &str, content: &str) -> std::path::PathBuf {
    let full_path = dir.join(rel_path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create dir");
    }
    std::fs::write(&full_path, content).expect("write file");
    full_path
}

/// Index `root`, a project holding only `file`, and return its entities.
fn index_file(file: &Path, root: &Path) -> Vec<GraphEntity> {
    let indexed = index_tree(root, true, 1);
    assert_eq!(indexed.files_parsed, 1, "{} is parsed", file.display());
    assert!(indexed.failed.is_empty(), "failed: {:?}", indexed.failed);
    indexed.entities
}

/// Count entities by node type.
fn count_nodes(entities: &[GraphEntity], node_type: &str) -> usize {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Node { node_type: nt, .. } if nt == node_type))
        .count()
}

/// Count entities by edge type.
fn count_edges(entities: &[GraphEntity], edge_type: &str) -> usize {
    entities
        .iter()
        .filter(|e| matches!(e, GraphEntity::Edge { edge_type: et, .. } if et == edge_type))
        .count()
}

/// The node of `node_type` whose slug or qualifiedName is `key` (a File's
/// slug is its path; a definition is found by its qualified name).
fn find_node<'a>(
    entities: &'a [GraphEntity],
    node_type: &str,
    key: &str,
) -> Option<&'a GraphEntity> {
    entities.iter().find(|e| match e {
        GraphEntity::Node {
            node_type: nt,
            slug,
            properties,
        } => {
            nt == node_type
                && (slug == key
                    || properties.get("qualifiedName").and_then(Value::as_str) == Some(key))
        }
        GraphEntity::Edge { .. } => false,
    })
}

/// Whether a Dependency node has the expected source.
fn has_dependency_with_source(entities: &[GraphEntity], expected_source: &str) -> bool {
    entities.iter().any(|e| match e {
        GraphEntity::Node {
            node_type,
            properties,
            ..
        } => {
            node_type == "Dependency"
                && properties.get("source").and_then(Value::as_str) == Some(expected_source)
        }
        GraphEntity::Edge { .. } => false,
    })
}

#[path = "ast_multi_language_extraction_test/extract_composer_dependencies.rs"]
mod extract_composer_dependencies;
#[path = "ast_multi_language_extraction_test/extract_cpp_functions_and.rs"]
mod extract_cpp_functions_and;
#[path = "ast_multi_language_extraction_test/extract_java_methods_and.rs"]
mod extract_java_methods_and;
#[path = "ast_multi_language_extraction_test/extract_kotlin_functions_and.rs"]
mod extract_kotlin_functions_and;
#[path = "ast_multi_language_extraction_test/extract_php_functions_and.rs"]
mod extract_php_functions_and;
#[path = "ast_multi_language_extraction_test/extract_python_functions_and.rs"]
mod extract_python_functions_and;
#[path = "ast_multi_language_extraction_test/walk_and_extract_finds.rs"]
mod walk_and_extract_finds;
