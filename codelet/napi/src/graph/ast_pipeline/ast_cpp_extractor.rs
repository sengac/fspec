//! C++ AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from C++ source files using text-based scanning.
//!
//! Note: ast-grep patterns like `$RET $NAME($$$ARGS) { $$$BODY }` don't match
//! C++ function_definition AST nodes, so we fall back to line-based scanning
//! for functions and use ast-grep patterns only for type declarations.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

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
pub fn extract_cpp(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Cpp;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test") || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "cpp", line_count, is_test,
    ));

    // Extract functions via line scanning (ast-grep patterns don't match C++ function_definition)
    extract_functions_by_scanning(source, &file_slug, &mut entities);

    // Extract types via ast-grep (these patterns work well)
    let root = lang.ast_grep(source);
    extract_types(&root, &file_slug, &mut entities);

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
fn extract_functions_by_scanning(
    source: &str,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();
    let lines: Vec<&str> = source.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip preprocessor, comments, blank lines
        if trimmed.starts_with('#')
            || trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.is_empty()
        {
            continue;
        }

        // Look for lines with ( and { that look like function definitions
        if let Some(name) = try_extract_cpp_func_name(trimmed) {
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            // Skip keywords
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
            // Find closing brace (approximate)
            let line_end = helpers::find_closing_brace(&lines, i).unwrap_or(i) as i32 + 1;

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
                is_public,
                param_count,
                line_start,
                line_end,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &fn_slug,
                "Contains",
            ));
        }
    }
}

/// Try to extract a function name from a C++ line that looks like a definition.
fn try_extract_cpp_func_name(line: &str) -> Option<String> {
    // Must have ( and { or end with {
    let has_paren = line.contains('(');
    let has_brace = line.contains('{');
    if !has_paren || !has_brace {
        return None;
    }

    // Skip lines that are control flow or declarations
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

    // The name is the last word before (
    let last_space = before.rfind(' ')?;
    let name = before[last_space + 1..]
        .trim_start_matches('*')
        .trim_start_matches('&');

    // Handle Class::method
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
