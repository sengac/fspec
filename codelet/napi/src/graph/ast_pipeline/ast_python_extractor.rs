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
    "def $NAME($$$ARGS) -> $RET: $$$BODY",
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
    let type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract import statements → Imports edges + import map
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

    // Extract TypeRef edges from function signatures (type annotations)
    extract_type_refs(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

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
                let resolved_path = match resolve_python_module(module_path, known_files) {
                    Some(p) => p,
                    None => continue,
                };

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

                if let Some(resolved_path) = resolve_python_module(module_path, known_files) {
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

/// Resolve a Python module path to a file path using suffix matching.
///
/// `click.core` → looks for any known file ending with `click/core.py`
/// Falls back to exact match for flat structures.
fn resolve_python_module(module_path: &str, known_files: &HashSet<String>) -> Option<String> {
    let suffix = module_path.replace('.', "/");
    let suffix_py = format!("{suffix}.py");
    let suffix_init = format!("{suffix}/__init__.py");

    // Try exact match first
    if known_files.contains(&suffix_py) {
        return Some(suffix_py);
    }
    if known_files.contains(&suffix_init) {
        return Some(suffix_init);
    }

    // Try suffix match: find any known file ending with /click/core.py
    let with_slash = format!("/{suffix_py}");
    if let Some(found) = known_files.iter().find(|f| f.ends_with(&with_slash)) {
        return Some(found.clone());
    }
    let with_slash_init = format!("/{suffix_init}");
    if let Some(found) = known_files.iter().find(|f| f.ends_with(&with_slash_init)) {
        return Some(found.clone());
    }

    None
}

/// Extract Calls edges from Python function bodies.
///
/// Scans each function body for bare function calls and resolves them
/// against known local functions and the import map.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
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
                    local_types,
                    import_map,
                    entities,
                );
            }
        }
    }
}

/// Extract TypeRef edges from Python function type annotations.
///
/// Python type annotations appear in function signatures:
/// - `def process(ctx: Context) -> None`
/// - `def foo(x: int, y: List[str]) -> Optional[Result]`
///
/// Filters out Python builtins (str, int, float, bool, None, etc.).
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Python;
    let root = lang.ast_grep(source);

    let python_builtins: HashSet<&str> = [
        "str", "int", "float", "bool", "bytes", "None", "list", "dict",
        "tuple", "set", "frozenset", "object", "type", "complex",
        "range", "slice", "memoryview", "bytearray",
        "List", "Dict", "Tuple", "Set", "FrozenSet", "Optional",
        "Union", "Any", "Callable", "Iterator", "Generator",
        "Sequence", "Mapping", "MutableMapping", "Iterable",
        "Type", "ClassVar", "Final", "Literal",
    ]
    .into_iter()
    .collect();

    for pattern in PYTHON_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = helpers::extract_name_after_keyword(&fn_text, "def ");
            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let fn_slug = format!("{file_slug}::{fn_name}");

            // Extract the signature (everything before the body colon)
            // For `def process(ctx: Context) -> None:`, we need everything up to the last `:`
            // Handle both `):` (no return type) and `) -> Type:` (with return type)
            let signature = if let Some(colon_pos) = fn_text.find("):") {
                &fn_text[..colon_pos + 1]
            } else if let Some(arrow_pos) = fn_text.find("->") {
                // Has return type: find the `)` before `->`
                if let Some(close_paren) = fn_text[..arrow_pos].rfind(')') {
                    &fn_text[..close_paren + 1]
                } else {
                    continue;
                }
            } else {
                continue;
            };

            let mut type_names = HashSet::new();
            extract_python_type_annotations(signature, &fn_text, &python_builtins, &mut type_names);

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

/// Extract type names from Python function signatures.
///
/// Looks for:
/// - Parameter annotations: `param: TypeName`
/// - Return type: `) -> TypeName:`
fn extract_python_type_annotations(
    signature: &str,
    full_text: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    // Extract parameter type annotations: `name: TypeName`
    // Look for patterns like `word: Word` inside the parameter list
    if let Some(paren_open) = signature.find('(') {
        let paren_close = signature.rfind(')').unwrap_or(signature.len());
        let params = &signature[paren_open + 1..paren_close];

        for param in params.split(',') {
            let param = param.trim();
            if let Some(colon_pos) = param.find(':') {
                let type_part = param[colon_pos + 1..].trim();
                // Extract the first identifier from the type annotation
                let type_name: String = type_part
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !type_name.is_empty() && !builtins.contains(type_name.as_str()) {
                    out.insert(type_name);
                }
            }
        }
    }

    // Extract return type annotation: `) -> TypeName`
    // Look in the full text for the return type between `)` and `:`
    if let Some(arrow_pos) = full_text.find("->") {
        let after_arrow = full_text[arrow_pos + 2..].trim();
        // Return type ends at `:` (start of body)
        let type_part = if let Some(colon_pos) = after_arrow.find(':') {
            after_arrow[..colon_pos].trim()
        } else {
            after_arrow
        };

        let type_name: String = type_part
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !type_name.is_empty() && !builtins.contains(type_name.as_str()) {
            out.insert(type_name);
        }
    }
}
