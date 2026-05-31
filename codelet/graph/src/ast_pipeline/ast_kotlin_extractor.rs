//! Kotlin AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from Kotlin source files
//! using kind-based AST matching.
//!
//! Uses `KindMatcher` to find `function_declaration` and type nodes rather than
//! pattern matching, which misses annotated functions (`@Test fun ...`),
//! functions with modifiers (`override fun ...`), and expression-body functions.

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// AST node kinds for Kotlin type declarations.
const KOTLIN_TYPE_KINDS: &[(&str, &str)] = &[
    ("class_declaration", "class"),
    ("object_declaration", "class"),
    ("interface_declaration", "interface"),
];

/// Kotlin standard library / JDK package prefixes that should NOT produce Imports edges.
const KOTLIN_EXTERNAL_PREFIXES: &[&str] = &[
    "java.", "javax.", "kotlin.", "android.", "org.junit", "org.jetbrains",
    "kotlinx.", "io.ktor",
];

/// Kotlin built-in types to filter from TypeRef edges.
const KOTLIN_BUILTIN_TYPES: &[&str] = &[
    "Int", "Long", "Short", "Byte", "Float", "Double", "Boolean", "Char",
    "String", "Unit", "Any", "Nothing", "List", "Map", "Set", "MutableList",
    "MutableMap", "MutableSet", "Array", "Pair", "Triple", "Sequence",
    "Iterable", "Collection", "Comparable", "Throwable", "Exception",
    "Void", "Object",
];

/// Extract entities from Kotlin source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
pub fn extract_kotlin(source: &str, rel_path: &str, known_files: &HashSet<String>) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Kotlin;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.kt")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "kotlin", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, lang, &mut entities);

    // Extract type declarations → collect names for TypeRef resolution
    let type_names = extract_types(&root, &file_slug, lang, &mut entities);

    // Extract import statements → collect import map for cross-file resolution
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(source, &file_slug, lang, &function_names, &type_names, &import_map, &mut entities);

    // Extract TypeRef edges from function signatures
    extract_type_refs(
        source, &file_slug, lang, &function_names, &type_names, &import_map, &mut entities,
    );

    // Extract top-level and class-level variables
    variables::extract_variables(source, &file_slug, rel_path, "kotlin", &mut entities);
    Ok(entities)
}

/// Extract function declarations from Kotlin source using kind-based matching.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    let matcher = match KindMatcher::try_new("function_declaration", lang) {
        Ok(m) => m,
        Err(_) => return seen_names,
    };

    for node in root.root().find_all(matcher) {
        let matched_text = node.text();
        let name = helpers::extract_name_after_keyword(&matched_text, "fun ");
        if name.is_empty() || !seen_names.insert(name.clone()) {
            continue;
        }

        let start_pos = node.start_pos();
        let end_pos = node.end_pos();
        let is_async = matched_text.contains("suspend fun ");
        let is_public = !matched_text.contains("private fun ")
            && !matched_text.contains("internal fun ");
        let param_count = helpers::count_params(&matched_text);

        let fn_slug = format!("{file_slug}::{name}");
        let cc = complexity::calculate(&matched_text, "kotlin");
            let meta = metadata::extract_function_meta(&matched_text, "kotlin");
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
            "kotlin",
            meta.truncated,
            ));

        entities.push(helpers::build_contains_edge(
            file_slug,
            &fn_slug,
            "Contains",
        ));
    }
    seen_names
}

/// Extract type declarations from Kotlin source using kind-based matching.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (kind_name, type_kind) in KOTLIN_TYPE_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let keyword = if *kind_name == "object_declaration" {
                "object "
            } else if matched_text.contains("interface ") {
                "interface "
            } else {
                "class "
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !matched_text.contains("private ")
                && !matched_text.contains("internal ");

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "kotlin");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "kotlin", type_meta.truncated,
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

/// Extract Kotlin `import` statements and produce Imports edges.
///
/// Resolves package paths to file paths: `com.myapp.service.UserService`
/// → `com/myapp/service/UserService.kt`. Only produces edges for imports
/// whose resolved path exists in `known_files`.
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

        if !trimmed.starts_with("import ") {
            continue;
        }

        let import_path = trimmed
            .strip_prefix("import ")
            .unwrap_or("")
            .trim();

        if import_path.is_empty() {
            continue;
        }

        // Skip external/standard library imports
        if KOTLIN_EXTERNAL_PREFIXES.iter().any(|p| import_path.starts_with(p)) {
            continue;
        }

        // Get local name (last segment)
        let local_name = import_path
            .rsplit('.')
            .next()
            .unwrap_or(import_path)
            .to_string();

        // Resolve to file path: dots → slashes + .kt
        let resolved_path = import_path.replace('.', "/") + ".kt";
        let is_local = known_files.contains(&resolved_path);

        if is_local {
            let target_slug = helpers::slugify_path(&resolved_path);
            import_map.insert(
                local_name.clone(),
                (target_slug.clone(), true, local_name.clone()),
            );

            edge_helpers::build_import_edge(
                file_slug,
                import_path,
                &resolved_path,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Extract Calls edges from Kotlin function bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    lang: SupportLang,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let root = lang.ast_grep(source);

    let matcher = match KindMatcher::try_new("function_declaration", lang) {
        Ok(m) => m,
        Err(_) => return,
    };

    for node in root.root().find_all(matcher) {
        let fn_text = node.text();
        let fn_name = helpers::extract_name_after_keyword(&fn_text, "fun ");
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

/// Extract TypeRef edges from Kotlin function signatures.
///
/// Kotlin signatures use `: Type` for parameter types and `: ReturnType` after `)`.
/// Example: `fun handle(req: Request): Response`
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    lang: SupportLang,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let root = lang.ast_grep(source);

    let matcher = match KindMatcher::try_new("function_declaration", lang) {
        Ok(m) => m,
        Err(_) => return,
    };

    let builtins: HashSet<&str> = KOTLIN_BUILTIN_TYPES.iter().copied().collect();

    for node in root.root().find_all(matcher) {
        let fn_text = node.text();
        let fn_name = helpers::extract_name_after_keyword(&fn_text, "fun ");
        if fn_name.is_empty() || !function_names.contains(&fn_name) {
            continue;
        }

        let fn_slug = format!("{file_slug}::{fn_name}");

        let signature = if let Some(brace_pos) = fn_text.find('{') {
            &fn_text[..brace_pos]
        } else {
            &fn_text
        };

        let mut type_names = HashSet::new();
        extract_kotlin_type_annotations(signature, &builtins, &mut type_names);

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

/// Extract type names from Kotlin function signatures.
///
/// Kotlin type annotations appear after `:` characters:
/// - Parameter types: `fun handle(req: Request)`
/// - Return type: `fun handle(): Response`
///
/// Filters out Kotlin built-in types.
fn extract_kotlin_type_annotations(
    signature: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    let bytes = signature.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b':' {
            i += 1;
            // Skip whitespace
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
            // Read type name
            if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let name = &signature[start..i];
                if !builtins.contains(name) {
                    out.insert(name.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
}
