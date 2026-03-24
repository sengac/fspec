//! Swift AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Swift source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Swift function declarations.
const SWIFT_FUNCTION_PATTERNS: &[&str] = &[
    "func $NAME($$$ARGS) -> $RET { $$$BODY }",
    "func $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep patterns for Swift type declarations.
const SWIFT_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("struct $NAME { $$$BODY }", "struct_kind"),
    ("protocol $NAME { $$$BODY }", "trait_kind"),
    ("enum $NAME { $$$BODY }", "enum_kind"),
];

/// Extract entities from Swift source code.
pub fn extract_swift(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Swift;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test")
        || rel_path.contains("test")
        || rel_path.contains("Tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "swift", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Swift source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in SWIFT_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "func ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = matched_text.starts_with("public ")
                || matched_text.starts_with("open ")
                || node
                    .parent()
                    .is_some_and(|p| p.text().starts_with("public ") || p.text().starts_with("open "));
            let is_async = matched_text.contains(" async ");
            let param_count = helpers::count_params(&matched_text);

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

/// Extract type declarations from Swift source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in SWIFT_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "struct_kind" => "struct ",
                "trait_kind" => "protocol ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = matched_text.starts_with("public ")
                || matched_text.starts_with("open ");

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
