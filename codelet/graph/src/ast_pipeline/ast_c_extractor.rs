//! C AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from C source files using ast-grep
//! pattern matching.
//!
//! C uses `#include "file.h"` for local imports and bare function calls.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph_entities::GraphEntity;

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

/// Extract entities from C source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
pub fn extract_c(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
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

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations
    extract_types(&root, source, &file_slug, &mut entities);

    // Extract #include directives → Imports edges
    let import_map = edge_helpers::extract_c_includes(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(&root, &file_slug, &function_names, &import_map, &mut entities);

    // Extract module-level variables
    variables::extract_variables(source, &file_slug, rel_path, "c", &mut entities);
    Ok(entities)
}

/// Extract function declarations from C source.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
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
            let is_public = !matched_text.trim_start().starts_with("static ");

            let fn_slug = format!("{file_slug}::{name}");
            let cc = complexity::calculate(&matched_text, "c");
            let meta = metadata::extract_function_meta(&matched_text, "c");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
                is_public,
                param_count,
                start_pos.line() as i32 + 1,
                end_pos.line() as i32 + 1,
            cc,
                &meta.parameters,
                &meta.source,
                &meta.docstring,
                &meta.decorators,
                "c",
                meta.truncated,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &fn_slug,
                "Contains",
            ));
        }
    }
    seen_names
}

/// Extract Calls edges from C function bodies.
fn extract_calls(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    for pattern in C_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = extract_c_function_name(&fn_text);
            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            if let Some(body_start) = fn_text.find('{') {
                let body = &fn_text[body_start..];
                let mut callee_names = HashSet::new();
                edge_helpers::extract_call_names_from_body(body, &mut callee_names);

                edge_helpers::resolve_calls(
                    &caller_slug,
                    file_slug,
                    &callee_names,
                    &fn_name,
                    local_functions,
                    &HashSet::new(),
                    import_map,
                    entities,
                );
            }
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
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "c");
            entities.push(helpers::build_type_node(
                file_slug, &name, "struct_kind", true,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "c", type_meta.truncated,
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
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "c");
            entities.push(helpers::build_type_node(
                file_slug, &name, "enum_kind", true,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "c", type_meta.truncated,
            ));
            entities.push(helpers::build_contains_edge(
                file_slug, &type_slug, "ContainsType",
            ));
        }
    }

    // Line-based fallback for structs/enums
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("struct ") && trimmed.contains('{') {
            let name = helpers::extract_name_after_keyword(trimmed, "struct ");
            if !name.is_empty() && seen_names.insert(name.clone()) {
                let type_slug = format!("{file_slug}::{name}");
                let type_meta = metadata::extract_type_meta(trimmed, "c");
                entities.push(helpers::build_type_node(
                    file_slug, &name, "struct_kind", true,
                    0, 0,
                    &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                    "c", type_meta.truncated,
                ));
                entities.push(helpers::build_contains_edge(file_slug, &type_slug, "ContainsType"));
            }
        } else if trimmed.starts_with("enum ") && trimmed.contains('{') {
            let name = helpers::extract_name_after_keyword(trimmed, "enum ");
            if !name.is_empty() && seen_names.insert(name.clone()) {
                let type_slug = format!("{file_slug}::{name}");
                let type_meta = metadata::extract_type_meta(trimmed, "c");
                entities.push(helpers::build_type_node(
                    file_slug, &name, "enum_kind", true,
                    0, 0,
                    &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                    "c", type_meta.truncated,
                ));
                entities.push(helpers::build_contains_edge(file_slug, &type_slug, "ContainsType"));
            }
        }
    }

    // Typedefs
    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("typedef ") {
            continue;
        }
        let name = extract_typedef_name(trimmed);
        if name.is_empty() || !seen_names.insert(name.clone()) {
            continue;
        }
        if matches!(name.as_str(), "struct" | "enum" | "int" | "void" | "char"
            | "unsigned" | "signed" | "long" | "short" | "double" | "float") {
            continue;
        }
        let type_slug = format!("{file_slug}::{name}");
        let type_meta = metadata::extract_type_meta(trimmed, "c");
        entities.push(helpers::build_type_node(
            file_slug, &name, "type_alias", true,
            0, 0,
            &type_meta.source, &type_meta.docstring, &type_meta.decorators,
            "c", type_meta.truncated,
        ));
        entities.push(helpers::build_contains_edge(file_slug, &type_slug, "ContainsType"));
    }
}

/// Extract C function name: the identifier immediately before `(`.
fn extract_c_function_name(text: &str) -> String {
    if let Some(paren_pos) = text.find('(') {
        let before = text[..paren_pos].trim();
        if let Some(last_space) = before.rfind(' ') {
            let name = before[last_space + 1..].trim_start_matches('*');
            return name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
        }
        return before
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
    }
    String::new()
}

/// Extract the typedef alias name (last identifier on the line).
fn extract_typedef_name(text: &str) -> String {
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
