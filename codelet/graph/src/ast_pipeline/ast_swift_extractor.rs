//! Swift AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Calls) from Swift source files using ast-grep
//! pattern matching.
//!
//! Swift uses module-level imports (`import Foundation`) which don't map to
//! individual files, so we skip Imports edges and focus on Calls edges only.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::complexity;
use super::metadata;
use super::variables;
use super::helpers;
use crate::graph_entities::GraphEntity;

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
///
/// Extracts File, Function, and Type nodes, plus Calls edges.
/// Swift uses module-level imports so Imports edges are not extracted.
pub fn extract_swift(source: &str, rel_path: &str, _known_files: &HashSet<String>) -> Result<Vec<GraphEntity>, String> {
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

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations
    extract_types(&root, &file_slug, &mut entities);

    // No import extraction for Swift (module-level only)
    let import_map: HashMap<String, (String, bool, String)> = HashMap::new();

    // Extract Calls edges from function bodies
    extract_calls(source, &file_slug, &function_names, &import_map, &mut entities);

    // Extract top-level and class-level variables
    variables::extract_variables(source, &file_slug, rel_path, "swift", &mut entities);
    Ok(entities)
}

/// Extract function declarations from Swift source.
///
/// Returns the set of function names found in this file (for call resolution).
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
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
            let cc = complexity::calculate(&matched_text, "swift");
            let meta = metadata::extract_function_meta(&matched_text, "swift");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                is_async,
                is_public,
                param_count,
                start_pos.line() as i32 + 1,
                end_pos.line() as i32 + 1,
            cc,
                &meta.parameters,
                &meta.source,
                &meta.docstring,
                &meta.decorators,
                "swift",
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
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "swift");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "swift", type_meta.truncated,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
}

/// Extract Calls edges from Swift function bodies.
///
/// Scans each function body for bare function calls and resolves them
/// against known local functions.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Swift;
    let root = lang.ast_grep(source);

    for pattern in SWIFT_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = helpers::extract_name_after_keyword(&fn_text, "func ");
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
