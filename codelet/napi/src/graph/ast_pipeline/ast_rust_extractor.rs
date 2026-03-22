//! Rust AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType) from Rust source files using ast-grep pattern matching.

use std::collections::HashSet;

use ast_grep_language::{LanguageExt, SupportLang};

use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Rust function declarations.
///
/// Needs separate patterns because `pub` changes the AST node structure.
/// Also needs with/without return type variants.
const RUST_FUNCTION_PATTERNS: &[(&str, bool)] = &[
    ("fn $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("fn $NAME($$$ARGS) { $$$BODY }", false),
    ("pub fn $NAME($$$ARGS) -> $RET { $$$BODY }", true),
    ("pub fn $NAME($$$ARGS) { $$$BODY }", true),
    ("pub async fn $NAME($$$ARGS) -> $RET { $$$BODY }", true),
    ("async fn $NAME($$$ARGS) -> $RET { $$$BODY }", false),
    ("pub async fn $NAME($$$ARGS) { $$$BODY }", true),
    ("async fn $NAME($$$ARGS) { $$$BODY }", false),
];

/// ast-grep patterns for Rust type declarations.
///
/// Each tuple: (pattern, type_kind, is_public_from_pattern).
const RUST_TYPE_PATTERNS: &[(&str, &str, bool)] = &[
    ("struct $NAME { $$$FIELDS }", "struct_kind", false),
    ("pub struct $NAME { $$$FIELDS }", "struct_kind", true),
    ("enum $NAME { $$$VARIANTS }", "enum_kind", false),
    ("pub enum $NAME { $$$VARIANTS }", "enum_kind", true),
    ("trait $NAME { $$$BODY }", "trait_kind", false),
    ("pub trait $NAME { $$$BODY }", "trait_kind", true),
];

/// Extract entities from Rust source code.
pub fn extract_rust(source: &str, rel_path: &str) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Rust;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    // Create File node
    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test") || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "rust", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations
    extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations (structs, enums, traits)
    extract_types(&root, &file_slug, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Rust source using multiple patterns.
///
/// Iterates over all function pattern variants (pub/non-pub, return/no-return, async/sync)
/// and deduplicates by function name.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, is_public_from_pattern) in RUST_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_fn_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.contains("async fn ");
            let is_public = *is_public_from_pattern
                || node
                    .parent()
                    .is_some_and(|p| p.text().starts_with("pub "));
            let param_count = helpers::count_params_rust(&matched_text);

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

/// Extract type declarations (struct, enum, trait) from Rust source.
///
/// Iterates over all type pattern variants and deduplicates by type name.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind, is_public_from_pattern) in RUST_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_type_name(&matched_text, type_kind);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = *is_public_from_pattern
                || node
                    .parent()
                    .is_some_and(|p| p.text().starts_with("pub "));

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug,
                &name,
                type_kind,
                is_public,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}

/// Extract function name from matched Rust text like "fn ensure_db(...) { ... }".
fn extract_fn_name(text: &str) -> String {
    super::helpers::extract_name_after_keyword(text, "fn ")
}

/// Extract type name from matched Rust text given the type kind keyword.
fn extract_type_name(text: &str, type_kind: &str) -> String {
    let keyword = match type_kind {
        "struct_kind" => "struct ",
        "enum_kind" => "enum ",
        "trait_kind" => "trait ",
        _ => return String::new(),
    };
    super::helpers::extract_name_after_keyword(text, keyword)
}
