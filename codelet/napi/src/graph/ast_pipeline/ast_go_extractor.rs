//! Go AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Go source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Go function declarations.
/// Each tuple: (pattern, has_receiver).
const GO_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("func $NAME($$$ARGS) $RET { $$$BODY }", false),
    ("func $NAME($$$ARGS) { $$$BODY }", false),
    ("func ($RECV) $NAME($$$ARGS) $RET { $$$BODY }", true),
    ("func ($RECV) $NAME($$$ARGS) { $$$BODY }", true),
];

/// ast-grep patterns for Go type declarations.
const GO_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("type $NAME struct { $$$FIELDS }", "struct_kind"),
    ("type $NAME interface { $$$METHODS }", "interface"),
];

/// Extract entities from Go source code.
pub fn extract_go(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Go;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.ends_with("_test.go");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "go", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Go source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, _has_receiver) in GO_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "func ");
            // For methods: "func (r *Recv) Name(...)" — skip past the receiver
            let name = if name.starts_with('(') {
                // Extract the name after the receiver closing paren
                if let Some(close) = matched_text.find(") ") {
                    let after = &matched_text[close + 2..];
                    helpers::extract_name_after_keyword(after, "")
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect()
                } else {
                    continue;
                }
            } else {
                name
            };

            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            // Go uses capitalization for public/private
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            let param_count = helpers::count_params_go(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false, // Go doesn't have async keyword
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

/// Extract type declarations from Go source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in GO_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "type ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = name.starts_with(|c: char| c.is_uppercase());

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
