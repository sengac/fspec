//! Scala AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Scala source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Scala function declarations.
const SCALA_FUNCTION_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $RET = { $$$BODY }",
    "def $NAME($$$ARGS) = { $$$BODY }",
    "def $NAME($$$ARGS): $RET = $BODY",
    "def $NAME($$$ARGS) = $BODY",
    "def $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep patterns for Scala type declarations.
const SCALA_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("case class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("case class $NAME($$$ARGS)", "class"),
    ("trait $NAME { $$$BODY }", "trait_kind"),
    ("object $NAME { $$$BODY }", "class"),
];

/// Extract entities from Scala source code.
pub fn extract_scala(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Scala;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test")
        || rel_path.contains("Spec")
        || rel_path.contains("test/")
        || rel_path.contains("spec/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "scala", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Scala source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in SCALA_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "def ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("protected ");
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
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

/// Extract type declarations from Scala source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in SCALA_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = if matched_text.contains("case class ") {
                "class "
            } else if matched_text.starts_with("object ") || matched_text.contains(" object ") {
                "object "
            } else if matched_text.contains("trait ") {
                "trait "
            } else {
                "class "
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("protected ");

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}
