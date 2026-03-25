//! Python AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from Python source files using
//! ast-grep pattern matching.
//!
//! Python import resolution converts dot-separated module paths to
//! slash-separated file paths + `.py`. Only project-local imports produce edges.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Python function declarations.
const PYTHON_FUNCTION_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $$$BODY",
];

/// ast-grep patterns for Python class declarations.
const PYTHON_CLASS_PATTERNS: &[&str] = &[
    "class $NAME($$$BASES): $$$BODY",
    "class $NAME: $$$BODY",
];

/// Extract entities from Python source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
/// The `known_files` set is used for import resolution — only modules that
/// exist as files in the project produce Imports edges.
pub fn extract_python(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Python;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test_")
        || rel_path.contains("_test.py")
        || rel_path.contains("tests/")
        || rel_path.contains("conftest");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "python", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type (class) declarations → collect names for TypeRef
    let _type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract import statements → Imports edges + import map
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(source, &file_slug, &function_names, &import_map, &mut entities);

    Ok(entities)
}

/// Extract function declarations from Python source.
///
/// Returns the set of function names found in this file (for call resolution).
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for pattern in PYTHON_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "def ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.starts_with("async ");
            let is_public = !name.starts_with('_');
            let param_count = helpers::count_params_python(&matched_text);

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
    seen_names
}

/// Extract class declarations from Python source.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for pattern in PYTHON_CLASS_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "class ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !name.starts_with('_');

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, "class", is_public,
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

/// Extract Python import statements and produce Imports edges.
///
/// Handles:
/// - `from click.core import BaseCommand` → resolves to `click/core.py`
/// - `import os.path` → resolves to `os/path.py` (skipped if not in known_files)
/// - `from .utils import helper` → relative import resolution
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

        if trimmed.starts_with("from ") {
            // `from module.path import Name1, Name2`
            if let Some((module_part, names_part)) = trimmed
                .strip_prefix("from ")
                .and_then(|s| s.split_once(" import "))
            {
                let module_path = module_part.trim();
                let resolved_path = resolve_python_module(module_path);

                let is_local = known_files.contains(&resolved_path);

                if is_local {
                    // Parse imported names
                    for name_item in names_part.split(',') {
                        let name_item = name_item.trim();
                        if name_item.is_empty() || name_item == "*" {
                            continue;
                        }

                        let (local_name, original_name) = if let Some((orig, alias)) =
                            name_item.split_once(" as ")
                        {
                            (alias.trim().to_string(), orig.trim().to_string())
                        } else {
                            (name_item.to_string(), name_item.to_string())
                        };

                        let target_slug = helpers::slugify_path(&resolved_path);
                        import_map.insert(
                            local_name,
                            (target_slug, true, original_name),
                        );
                    }

                    edge_helpers::build_import_edge(
                        file_slug,
                        module_path,
                        &resolved_path,
                        false,
                        entities,
                    );
                }
            }
        } else if trimmed.starts_with("import ") {
            // `import module.path` or `import module.path as alias`
            let import_part = trimmed.strip_prefix("import ").unwrap_or("").trim();

            for module_item in import_part.split(',') {
                let module_item = module_item.trim();
                if module_item.is_empty() {
                    continue;
                }

                let (module_path, _local_name) = if let Some((mod_path, alias)) =
                    module_item.split_once(" as ")
                {
                    (mod_path.trim(), alias.trim().to_string())
                } else {
                    (module_item, module_item.to_string())
                };

                let resolved_path = resolve_python_module(module_path);
                let is_local = known_files.contains(&resolved_path);

                if is_local {
                    edge_helpers::build_import_edge(
                        file_slug,
                        module_path,
                        &resolved_path,
                        false,
                        entities,
                    );
                }
            }
        }
    }
    import_map
}

/// Resolve a Python module path to a file path.
///
/// `click.core` → `click/core.py`
/// `.utils` → relative (handled by caller)
fn resolve_python_module(module_path: &str) -> String {
    let path = module_path.replace('.', "/");
    format!("{path}.py")
}

/// Extract Calls edges from Python function bodies.
///
/// Scans each function body for bare function calls and resolves them
/// against known local functions and the import map.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Python;
    let root = lang.ast_grep(source);

    for pattern in PYTHON_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = helpers::extract_name_after_keyword(&fn_text, "def ");
            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Python function body starts after the colon on the def line
            // Find the body by locating the first colon after the closing paren
            if let Some(colon_pos) = fn_text.find("):") {
                let body = &fn_text[colon_pos + 2..];

                let mut callee_names = HashSet::new();
                edge_helpers::extract_call_names_from_body(body, &mut callee_names);

                edge_helpers::resolve_calls(
                    &caller_slug,
                    file_slug,
                    &callee_names,
                    &fn_name,
                    local_functions,
                    import_map,
                    entities,
                );
            }
        }
    }
}
