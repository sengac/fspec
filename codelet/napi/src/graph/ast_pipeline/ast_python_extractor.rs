//! Python AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Python source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Python function declarations.
/// Note: "def $NAME($$$ARGS): $$$BODY" also matches async defs in Python's tree-sitter,
/// so we only use this one pattern and check for async via text.
const PYTHON_FUNCTION_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $$$BODY",
];

/// ast-grep patterns for Python class declarations.
const PYTHON_CLASS_PATTERNS: &[&str] = &[
    "class $NAME($$$BASES): $$$BODY",
    "class $NAME: $$$BODY",
];

/// Extract entities from Python source code.
pub fn extract_python(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Python;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test_")
        || rel_path.contains("_test.py")
        || rel_path.contains("tests/")
        || rel_path.contains("conftest");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "python", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Python source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in PYTHON_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "def ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.starts_with("async ");
            // In Python, names starting with _ are private by convention
            let is_public = !name.starts_with('_');
            let param_count = helpers::count_params_python(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                is_async,
                is_public,
                param_count,
                start_pos.line() as i32 + 1,
                end_pos.line() as i32 + 1,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &fn_slug,
                "Contains",
            ));
        }
    }
}

/// Extract class declarations from Python source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in PYTHON_CLASS_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "class ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !name.starts_with('_');

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, "class", is_public,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}
