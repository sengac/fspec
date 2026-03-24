//! Kotlin AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Kotlin source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Kotlin function declarations.
/// Each tuple: (pattern, is_async).
const KOTLIN_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("fun $NAME($$$ARGS): $RET { $$$BODY }", false),
    ("fun $NAME($$$ARGS) { $$$BODY }", false),
    ("suspend fun $NAME($$$ARGS): $RET { $$$BODY }", true),
    ("suspend fun $NAME($$$ARGS) { $$$BODY }", true),
];

/// ast-grep patterns for Kotlin type declarations.
/// Each tuple: (pattern, type_kind, is_public).
const KOTLIN_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("data class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("data class $NAME($$$ARGS)", "class"),
    ("interface $NAME { $$$BODY }", "interface"),
    ("object $NAME { $$$BODY }", "class"),
    ("enum class $NAME { $$$BODY }", "enum_kind"),
];

/// Extract entities from Kotlin source code.
pub fn extract_kotlin(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Kotlin;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.kt")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "kotlin", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Kotlin source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, is_async) in KOTLIN_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "fun ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("internal ");
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                *is_async,
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

/// Extract type declarations from Kotlin source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in KOTLIN_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            // For "data class", "enum class", "object" etc, use correct keyword
            let keyword = if matched_text.contains("enum class ") || matched_text.contains("data class ") {
                "class "
            } else if matched_text.starts_with("object ") {
                "object "
            } else if matched_text.contains("interface ") {
                "interface "
            } else {
                "class "
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("internal ");

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
