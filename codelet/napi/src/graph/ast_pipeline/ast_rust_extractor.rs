//! Rust AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from Rust source files
//! using ast-grep pattern matching.
//!
//! Rust imports use `use crate::`, `use super::`, and `mod` statements.
//! External crate imports are filtered out — only project-local references
//! produce edges.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
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
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
/// The `known_files` set is used for import resolution — only Rust files that exist
/// in the project produce Imports edges (filtering out external crates).
pub fn extract_rust(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
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

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations → collect names for TypeRef resolution
    let type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract import statements → collect import map for cross-file resolution
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

    // Extract TypeRef edges from function signatures
    extract_type_refs(
        source,
        &file_slug,
        &function_names,
        &type_names,
        &import_map,
        &mut entities,
    );

    // Extract module-level variables
    variables::extract_variables(source, &file_slug, rel_path, "rust", &mut entities);
    Ok(entities)
}

/// Extract function declarations from Rust source using multiple patterns.
///
/// Iterates over all function pattern variants (pub/non-pub, return/no-return, async/sync)
/// and deduplicates by function name. Returns the set of function names.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
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
            let cc = complexity::calculate(&matched_text, "rust");
            let meta = metadata::extract_function_meta(&matched_text, "rust");

            let fn_slug = format!("{file_slug}::{name}");
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
                "rust",
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

/// Extract type declarations (struct, enum, trait) from Rust source.
///
/// Iterates over all type pattern variants and deduplicates by type name.
/// Returns the set of type names.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
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
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "rust");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "rust", type_meta.truncated,
            ));

            entities.push(helpers::build_contains_edge(
                file_slug,
                &type_slug,
                "ContainsType",
            ));
        }
    }
    seen_names
}

/// Extract Rust `use` statements and produce Imports edges.
///
/// Resolves `use crate::path::module;` to file paths like `path/module.rs`.
/// Only produces edges for imports whose resolved path exists in `known_files`.
/// External crate imports (anything not starting with `crate::`, `super::`, or `self::`)
/// are filtered out.
///
/// Returns a map of `local_name → (target_file_slug, is_local, original_name)`.
fn extract_imports(
    source: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();

        if !trimmed.starts_with("use ") {
            continue;
        }

        // Strip `use ` prefix and `;` suffix
        let import_part = trimmed
            .strip_prefix("use ")
            .unwrap_or("")
            .trim_end_matches(';')
            .trim();

        if import_part.is_empty() {
            continue;
        }

        // Only handle crate-local imports
        let is_crate_local = import_part.starts_with("crate::")
            || import_part.starts_with("super::")
            || import_part.starts_with("self::");

        if !is_crate_local {
            continue;
        }

        // Strip the prefix to get the module path
        let module_path = if let Some(rest) = import_part.strip_prefix("crate::") {
            rest
        } else if let Some(rest) = import_part.strip_prefix("super::") {
            rest
        } else if let Some(rest) = import_part.strip_prefix("self::") {
            rest
        } else {
            continue;
        };

        // Get the local name (last segment, possibly after `::`)
        let local_name = module_path
            .rsplit("::")
            .next()
            .unwrap_or(module_path)
            .to_string();

        // Resolve to file path: convert `::` to `/` and try .rs extension
        let resolved_path = resolve_rust_module_path(module_path, known_files);

        if let Some(resolved) = resolved_path {
            let target_slug = helpers::slugify_path(&resolved);
            import_map.insert(
                local_name.clone(),
                (target_slug.clone(), true, local_name.clone()),
            );

            edge_helpers::build_import_edge(
                file_slug,
                import_part,
                &resolved,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Resolve a Rust module path to a file path against known_files.
///
/// Tries multiple resolution strategies:
/// - `path/to/module.rs`
/// - `path/to/module/mod.rs`
/// - `src/path/to/module.rs`
/// - `src/path/to/module/mod.rs`
fn resolve_rust_module_path(module_path: &str, known_files: &HashSet<String>) -> Option<String> {
    let fs_path = module_path.replace("::", "/");

    let candidates = [
        format!("{fs_path}.rs"),
        format!("{fs_path}/mod.rs"),
        format!("src/{fs_path}.rs"),
        format!("src/{fs_path}/mod.rs"),
    ];

    for candidate in &candidates {
        if known_files.contains(candidate.as_str()) {
            return Some(candidate.clone());
        }
    }
    None
}

/// Extract Calls edges from Rust function bodies.
///
/// For each function, finds bare function calls and resolves them
/// against local functions and the import map.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Rust;
    let root = lang.ast_grep(source);

    for (pattern, _) in RUST_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = extract_fn_name(&fn_text);
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
                    local_types,
                    import_map,
                    entities,
                );
            }
        }
    }
}

/// Extract TypeRef edges from Rust function signatures.
///
/// Parses type annotations after `:` and `->` in function signatures.
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Rust;
    let root = lang.ast_grep(source);

    for (pattern, _) in RUST_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = extract_fn_name(&fn_text);
            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let fn_slug = format!("{file_slug}::{fn_name}");

            // Extract signature: everything before the first {
            let signature = if let Some(brace_pos) = fn_text.find('{') {
                &fn_text[..brace_pos]
            } else {
                &fn_text
            };

            let mut type_names = HashSet::new();
            extract_rust_type_annotations(signature, &mut type_names);

            edge_helpers::resolve_type_refs(
                &fn_slug,
                file_slug,
                &type_names,
                local_types,
                import_map,
                entities,
            );
        }
    }
}

/// Extract type names from Rust function signatures.
///
/// Finds types after `:` (parameter types) and `->` (return types).
/// Filters out primitives and standard library types.
fn extract_rust_type_annotations(signature: &str, out: &mut HashSet<String>) {
    let rust_builtins: HashSet<&str> = [
        "str", "String", "bool", "i8", "i16", "i32", "i64", "i128",
        "u8", "u16", "u32", "u64", "u128", "f32", "f64", "usize", "isize",
        "char", "Self", "self", "Option", "Result", "Vec", "Box", "Arc",
        "Rc", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "Cow",
    ]
    .into_iter()
    .collect();

    // Extract return type after ->
    if let Some(arrow_pos) = signature.find("->") {
        let after_arrow = signature[arrow_pos + 2..].trim();
        extract_type_name_from_annotation(after_arrow, &rust_builtins, out);
    }

    // Extract parameter types after : in the parameter list
    if let Some(paren_open) = signature.find('(') {
        let paren_close = signature.rfind(')').unwrap_or(signature.len());
        let params_str = &signature[paren_open + 1..paren_close];

        for param in params_str.split(',') {
            let param = param.trim();
            if let Some(colon_pos) = param.find(':') {
                let type_part = param[colon_pos + 1..].trim();
                // Skip references: &str, &mut Type → extract Type
                let type_part = type_part
                    .trim_start_matches('&')
                    .trim_start_matches("mut ")
                    .trim();
                extract_type_name_from_annotation(type_part, &rust_builtins, out);
            }
        }
    }
}

/// Extract a type name from a Rust type annotation fragment.
///
/// Handles generic wrappers: `Vec<GraphEntity>` → `GraphEntity`,
/// `Option<String>` → skip (String is builtin).
fn extract_type_name_from_annotation(
    type_str: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    // Handle generic types: Vec<Type>, Option<Type>, Result<T, E>
    if let Some(angle_pos) = type_str.find('<') {
        let outer = type_str[..angle_pos].trim();
        let inner = type_str[angle_pos + 1..].trim_end_matches('>').trim();

        // Check the outer type
        if !outer.is_empty() && !builtins.contains(outer) {
            let name: String = outer
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && !builtins.contains(name.as_str()) {
                out.insert(name);
            }
        }

        // Check inner types (split by comma for Result<T, E>)
        for inner_type in inner.split(',') {
            let inner_type = inner_type.trim();
            extract_type_name_from_annotation(inner_type, builtins, out);
        }
    } else {
        // Plain type name
        let name: String = type_str
            .trim_start_matches('&')
            .trim_start_matches("mut ")
            .trim()
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() && !builtins.contains(name.as_str()) {
            out.insert(name);
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
