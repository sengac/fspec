//! C++ AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from C++ source files.
//!
//! Uses text-based scanning for functions (ast-grep patterns don't match C++
//! function_definition nodes) and `#include` parsing for imports.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for C++ class/struct/enum declarations.
const CPP_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("struct $NAME { $$$BODY }", "struct_kind"),
    ("enum $NAME { $$$VARIANTS }", "enum_kind"),
    ("namespace $NAME { $$$BODY }", "interface"),
];

/// Extract entities from C++ source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
pub fn extract_cpp(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Cpp;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test") || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "cpp", line_count, is_test,
    ));

    // Extract functions via line scanning (ast-grep patterns don't match C++ function_definition)
    let function_names = extract_functions_by_scanning(source, &file_slug, &mut entities);

    // Extract types via ast-grep (these patterns work well)
    let root = lang.ast_grep(source);
    extract_types(&root, &file_slug, &mut entities);

    // Extract #include directives → Imports edges (shared with C)
    let import_map = edge_helpers::extract_c_includes(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls_by_scanning(source, &file_slug, &function_names, &import_map, &mut entities);

    Ok(entities)
}

/// Detect whether a `.h` file is C++ based on content heuristics.
pub fn is_cpp_header(source: &str) -> bool {
    source.contains("class ")
        || source.contains("namespace ")
        || source.contains("template")
        || source.contains("std::")
        || source.contains("public:")
        || source.contains("private:")
        || source.contains("protected:")
        || source.contains("#include <string>")
        || source.contains("#include <vector>")
        || source.contains("#include <map>")
        || source.contains("#include <iostream>")
}

/// Extract functions by scanning lines for `type name(params) {` patterns.
///
/// Returns the set of function names found.
fn extract_functions_by_scanning(
    source: &str,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if trimmed.starts_with('#')
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.is_empty()
        {
            continue;
        }

        if let Some(name) = try_extract_cpp_func_name(trimmed) {
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            if matches!(
                name.as_str(),
                "if" | "for" | "while" | "switch" | "return"
                    | "struct" | "class" | "enum" | "namespace" | "typedef"
                    | "using" | "catch" | "else"
            ) {
                continue;
            }

            let is_public = !trimmed.starts_with("private ");
            let param_count = helpers::count_params(trimmed);
            let line_start = i as i32 + 1;
            let line_end = helpers::find_closing_brace(&lines, i).unwrap_or(i) as i32 + 1;

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug, &name, false, is_public, param_count, line_start, line_end,
            ));

            entities.push(helpers::build_contains_edge(file_slug, &fn_slug, "Contains"));
        }
    }
    seen_names
}

/// Extract Calls edges from C++ function bodies using line scanning.
fn extract_calls_by_scanning(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(fn_name) = try_extract_cpp_func_name(trimmed) {
            if fn_name.is_empty() || local_functions.contains(&fn_name) {
                // This is a function definition line — extract body
                let caller_slug = format!("{file_slug}::{fn_name}");
                let end = helpers::find_closing_brace(&lines, i).unwrap_or(i);
                if end > i {
                    let body: String = lines[i..=end].join("\n");
                    if let Some(body_start) = body.find('{') {
                        let fn_body = &body[body_start..];
                        let mut callee_names = HashSet::new();
                        edge_helpers::extract_call_names_from_body(fn_body, &mut callee_names);

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
    }
}

/// Try to extract a function name from a C++ line.
fn try_extract_cpp_func_name(line: &str) -> Option<String> {
    let has_paren = line.contains('(');
    let has_brace = line.contains('{');
    if !has_paren || !has_brace {
        return None;
    }

    if line.starts_with("if ")
        || line.starts_with("for ")
        || line.starts_with("while ")
        || line.starts_with("switch ")
        || line.starts_with("class ")
        || line.starts_with("struct ")
        || line.starts_with("enum ")
        || line.starts_with("namespace ")
    {
        return None;
    }

    let paren_pos = line.find('(')?;
    let before = line[..paren_pos].trim();
    if before.is_empty() {
        return None;
    }

    let last_space = before.rfind(' ')?;
    let name = before[last_space + 1..]
        .trim_start_matches('*')
        .trim_start_matches('&');

    let name = if let Some(colon_pos) = name.rfind("::") {
        &name[colon_pos + 2..]
    } else {
        name
    };

    if name.is_empty() || !name.chars().next()?.is_alphabetic() {
        return None;
    }

    Some(
        name.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect(),
    )
}

/// Extract type declarations from C++ source via ast-grep.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in CPP_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "struct_kind" => "struct ",
                "enum_kind" => "enum ",
                "interface" => "namespace ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(file_slug, &name, type_kind, true));
            entities.push(helpers::build_contains_edge(file_slug, &type_slug, "ContainsType"));
        }
    }
}
