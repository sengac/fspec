//! C# AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from C# source files.
//!
//! Uses line-based scanning for methods (ast-grep patterns don't match C#
//! method_declaration nodes) and ast-grep patterns for type declarations.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for C# type declarations.
const CSHARP_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("public class $NAME { $$$BODY }", "class", true),
    ("class $NAME { $$$BODY }", "class", false),
    ("public interface $NAME { $$$BODY }", "interface", true),
    ("interface $NAME { $$$BODY }", "interface", false),
    ("public struct $NAME { $$$BODY }", "struct_kind", true),
    ("struct $NAME { $$$BODY }", "struct_kind", false),
    ("public enum $NAME { $$$BODY }", "enum_kind", true),
    ("enum $NAME { $$$BODY }", "enum_kind", false),
];

/// Extract entities from C# source code.
pub fn extract_csharp(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::CSharp;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test")
        || rel_path.contains("test")
        || rel_path.contains("Tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "csharp", line_count, is_test,
    ));

    // Methods via line scanning
    extract_methods_by_scanning(source, &file_slug, &mut entities);

    // Types via ast-grep
    let root = lang.ast_grep(source);
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract methods by scanning lines for access_modifier return_type name(params) { patterns.
fn extract_methods_by_scanning(
    source: &str,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();
    let lines: Vec<&str> = source.lines().collect();
    let access_modifiers = ["public", "private", "protected", "internal"];

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Must have ( and { to be a method definition
        if !trimmed.contains('(') || !trimmed.contains('{') {
            continue;
        }

        // Skip non-method lines
        if trimmed.starts_with("if ")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("switch ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("public class ")
            || trimmed.starts_with("namespace ")
            || trimmed.starts_with("//")
        {
            continue;
        }

        // Check if line starts with access modifier or known patterns
        let mut words: Vec<&str> = trimmed.split_whitespace().collect();

        // Must have at least 3 tokens: modifier? return_type name(
        // or at least: return_type name(
        if words.len() < 2 {
            continue;
        }

        // Remove access modifier and determine visibility
        let is_public = access_modifiers.contains(&words[0]) && words[0] == "public";
        if access_modifiers.contains(&words[0]) {
            words.remove(0);
        }

        // Handle "static", "async", "virtual", "override" etc modifiers
        let is_async = words.contains(&"async");
        words.retain(|w| {
            !matches!(
                *w,
                "static" | "async" | "virtual" | "override" | "abstract" | "sealed"
            )
        });

        // Now expect: return_type name(params) { ... }
        if words.len() < 2 {
            continue;
        }

        // Find the token that contains (
        let name_token_idx = words
            .iter()
            .position(|w| w.contains('('));
        let name_token_idx = match name_token_idx {
            Some(idx) if idx >= 1 => idx,
            _ => continue,
        };

        let name_part = words[name_token_idx];
        let paren_pos = match name_part.find('(') {
            Some(p) => p,
            None => continue,
        };
        let name: String = name_part[..paren_pos]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();

        if name.is_empty() || !seen_names.insert(name.clone()) {
            continue;
        }

        // Skip if it looks like a type declaration
        if matches!(name.as_str(), "class" | "struct" | "interface" | "enum") {
            continue;
        }

        let param_count = helpers::count_params(trimmed);
        let line_start = i as i32 + 1;
        let line_end = helpers::find_closing_brace(&lines, i).unwrap_or(i) as i32 + 1;

        let fn_slug = format!("{file_slug}::{name}");
        entities.push(helpers::build_function_node(
            file_slug,
            &name,
            is_async,
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

/// Extract type declarations from C# source via ast-grep.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind, is_public_from_pattern) in CSHARP_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => "class ",
                "interface" => "interface ",
                "struct_kind" => "struct ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, *is_public_from_pattern,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}
