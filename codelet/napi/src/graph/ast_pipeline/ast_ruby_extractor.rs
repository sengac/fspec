//! Ruby AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Ruby source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Ruby method declarations.
const RUBY_METHOD_PATTERNS: &[(&str, bool)] = &[
    ("def $NAME($$$ARGS) $$$BODY end", false),
    ("def $NAME $$$BODY end", false),
    ("def self.$NAME($$$ARGS) $$$BODY end", true),
    ("def self.$NAME $$$BODY end", true),
];

/// ast-grep patterns for Ruby class/module declarations.
const RUBY_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME $$$BODY end", "class"),
    ("module $NAME $$$BODY end", "interface"),
];

/// Extract entities from Ruby source code.
pub fn extract_ruby(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Ruby;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("_spec.rb")
        || rel_path.contains("_test.rb")
        || rel_path.contains("test/")
        || rel_path.contains("spec/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "ruby", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_methods(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract method declarations from Ruby source.
fn extract_methods(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, is_class_method) in RUBY_METHOD_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = if *is_class_method {
                // "def self.foo" → extract after "self."
                if let Some(dot_pos) = matched_text.find("self.") {
                    let after = &matched_text[dot_pos + 5..];
                    after
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '?' || *c == '!')
                        .collect::<String>()
                } else {
                    continue;
                }
            } else {
                helpers::extract_name_after_keyword(&matched_text, "def ")
            };

            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            // In Ruby, methods starting with _ are considered private
            let is_public = !name.starts_with('_');
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

/// Extract class/module declarations from Ruby source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in RUBY_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "interface" => "module ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, true,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}
