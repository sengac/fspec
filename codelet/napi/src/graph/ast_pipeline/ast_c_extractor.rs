//! C AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from C source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for C function declarations.
const C_FUNCTION_PATTERNS: &[&str] = &[
    "static $RET $NAME($$$ARGS) { $$$BODY }",
    "$RET $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep patterns for C struct declarations.
const C_STRUCT_PATTERNS: &[&str] = &[
    "struct $NAME { $$$FIELDS }",
];

/// ast-grep patterns for C enum declarations.
const C_ENUM_PATTERNS: &[&str] = &[
    "enum $NAME { $$$VARIANTS }",
];

/// ast-grep patterns for C typedef declarations.
/// Note: tree-sitter's C grammar may not always match these patterns,
/// so we also have line-based fallback in extract_types.

/// Extract entities from C source code.
pub fn extract_c(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::C;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test") || rel_path.contains("tests/");
    let language = if rel_path.ends_with(".h") { "c-header" } else { "c" };

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, language, line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    extract_functions(&root, &file_slug, &mut entities);
    extract_types(&root, source, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from C source.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in C_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_c_function_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            // Skip type keywords that look like functions
            if matches!(name.as_str(), "if" | "for" | "while" | "switch" | "return" | "struct" | "enum" | "typedef") {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let param_count = helpers::count_params(&matched_text);
            // In C, `static` functions have file scope (private)
            let is_public = !matched_text.trim_start().starts_with("static ");

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

/// Extract type declarations from C source.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    source: &str,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for pattern in C_STRUCT_PATTERNS {
        let struct_matches: Vec<_> = root.root().find_all(*pattern).collect();
        for node in struct_matches {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "struct ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }
            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, "struct_kind", true,
            ));
            entities.push(helpers::build_contains_edge(
                file_slug, &type_slug, "ContainsType",
            ));
        }
    }

    for pattern in C_ENUM_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "enum ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }
            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, "enum_kind", true,
            ));
            entities.push(helpers::build_contains_edge(
                file_slug, &type_slug, "ContainsType",
            ));
        }
    }

    // Line-based fallback for structs/enums that ast-grep patterns miss
    // (tree-sitter C grammar may not match patterns with trailing semicolons)
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("struct ") && trimmed.contains('{') {
            let name = helpers::extract_name_after_keyword(trimmed, "struct ");
            if !name.is_empty() && seen_names.insert(name.clone()) {
                let type_slug = format!("{file_slug}::{name}");
                entities.push(helpers::build_type_node(
                    file_slug, &name, "struct_kind", true,
                ));
                entities.push(helpers::build_contains_edge(
                    file_slug, &type_slug, "ContainsType",
                ));
            }
        } else if trimmed.starts_with("enum ") && trimmed.contains('{') {
            let name = helpers::extract_name_after_keyword(trimmed, "enum ");
            if !name.is_empty() && seen_names.insert(name.clone()) {
                let type_slug = format!("{file_slug}::{name}");
                entities.push(helpers::build_type_node(
                    file_slug, &name, "enum_kind", true,
                ));
                entities.push(helpers::build_contains_edge(
                    file_slug, &type_slug, "ContainsType",
                ));
            }
        }
    }

    // Typedefs — use line-based detection for reliable extraction
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("typedef ") {
            continue;
        }
        let name = extract_typedef_name(trimmed);
        if name.is_empty() || !seen_names.insert(name.clone()) {
            continue;
        }
        // Skip keywords and well-known type names
        if matches!(name.as_str(), "struct" | "enum" | "int" | "void" | "char"
            | "unsigned" | "signed" | "long" | "short" | "double" | "float") {
            continue;
        }
        let type_slug = format!("{file_slug}::{name}");
        entities.push(helpers::build_type_node(
            file_slug, &name, "typedef", true,
        ));
        entities.push(helpers::build_contains_edge(
            file_slug, &type_slug, "ContainsType",
        ));
    }
}

/// Extract C function name: the identifier immediately before `(`.
fn extract_c_function_name(text: &str) -> String {
    if let Some(paren_pos) = text.find('(') {
        let before = text[..paren_pos].trim();
        // Last token before ( — could be "*name" for pointer returns
        if let Some(last_space) = before.rfind(' ') {
            let name = before[last_space + 1..].trim_start_matches('*');
            return name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
        }
        // No space means the whole thing is the name (unlikely in real C)
        return before
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
    }
    String::new()
}

/// Extract the typedef alias name (last identifier on the line).
fn extract_typedef_name(text: &str) -> String {
    // "typedef struct Point PointT;" → "PointT"
    let trimmed = text.trim().trim_end_matches(';');
    if let Some(last_space) = trimmed.rfind(' ') {
        return trimmed[last_space + 1..]
            .trim_start_matches('*')
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
    }
    String::new()
}
