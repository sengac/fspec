//! Java AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Java source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Java method declarations.
/// Each tuple: (pattern, is_public_from_pattern).
const JAVA_METHOD_PATTERNS: &[(&str, bool)] = &[
    ("public $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("public static $RET $NAME($$$ARGS) { $$$BODY }", true),
    ("private $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("protected $RET $NAME($$$ARGS) { $$$BODY }", false),
    ("$RET $NAME($$$ARGS) { $$$BODY }", false),
];

/// ast-grep patterns for Java type declarations.
/// Each tuple: (pattern, type_kind, is_public).
const JAVA_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("public class $NAME { $$$BODY }", "class", true),
    ("class $NAME { $$$BODY }", "class", false),
    ("public interface $NAME { $$$BODY }", "interface", true),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("public enum $NAME { $$$BODY }", "enum_kind", true),
    ("enum $NAME { $$$BODY }", "enum_kind", false),
];

/// Extract entities from Java source code.
pub fn extract_java(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Java;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.java")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "java", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_methods(&root, &file_slug, &mut entities);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract method declarations from Java source.
fn extract_methods(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, is_public_from_pattern) in JAVA_METHOD_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            // Find name: it's the identifier just before (
            let name = extract_java_method_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = *is_public_from_pattern
                || matched_text.starts_with("public ");
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

/// Extract type declarations from Java source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind, is_public_from_pattern) in JAVA_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "interface" => "interface ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = *is_public_from_pattern;

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

/// Extract Java method name: the identifier immediately before `(`.
fn extract_java_method_name(text: &str) -> String {
    if let Some(paren_pos) = text.find('(') {
        let before = text[..paren_pos].trim();
        // Last word before (
        if let Some(last_space) = before.rfind(' ') {
            let name = &before[last_space + 1..];
            return name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
        }
    }
    String::new()
}
