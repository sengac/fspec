//! PHP AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from PHP source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for PHP function declarations.
/// Each tuple: (pattern, is_public).
const PHP_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("function $NAME($$$ARGS) { $$$BODY }", true),
    ("public function $NAME($$$ARGS) { $$$BODY }", true),
    ("public static function $NAME($$$ARGS) { $$$BODY }", true),
    ("private function $NAME($$$ARGS) { $$$BODY }", false),
    ("protected function $NAME($$$ARGS) { $$$BODY }", false),
    ("static function $NAME($$$ARGS) { $$$BODY }", true),
];

/// ast-grep patterns for PHP type declarations.
/// Each tuple: (pattern, type_kind, is_public).
const PHP_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("interface $NAME { $$$BODY }", "interface"),
    ("trait $NAME { $$$BODY }", "trait_kind"),
    ("enum $NAME { $$$BODY }", "enum_kind"),
];

/// Extract entities from PHP source code.
pub fn extract_php(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Php;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.php")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "php", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function/method declarations from PHP source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, is_public_from_pattern) in PHP_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "function ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = *is_public_from_pattern;
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

/// Extract type declarations from PHP source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in PHP_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "interface" => "interface ",
                "trait_kind" => "trait ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, true, // PHP classes are public by default
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}
