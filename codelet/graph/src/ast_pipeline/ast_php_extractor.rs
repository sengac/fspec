//! PHP AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from PHP source files
//! using kind-based AST matching.
//!
//! Uses `KindMatcher` to find `method_declaration` and `function_definition` nodes
//! rather than pattern matching, which fails for PHP class methods due to AST
//! structure differences (visibility modifiers, return types, annotations).

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph_entities::GraphEntity;

/// AST node kinds for PHP functions/methods.
const PHP_FUNC_KINDS: &[&str] = &["method_declaration", "function_definition"];

/// AST node kinds for PHP type declarations.
const PHP_TYPE_KINDS: &[&str] = &[
    "class_declaration",
    "interface_declaration",
    "trait_declaration",
    "enum_declaration",
];

/// Extract entities from PHP source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
/// The `known_files` set is used for import resolution — only PHP files that exist
/// in the project produce Imports edges (filtering out external packages).
pub fn extract_php(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Php;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.php")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "php", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function/method declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, lang, &mut entities);

    // Extract type declarations → collect names for TypeRef resolution
    let type_names = extract_types(&root, &file_slug, lang, &mut entities);

    // Extract import statements → collect import map for cross-file resolution
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function/method bodies
    extract_calls(
        source,
        &file_slug,
        lang,
        &function_names,
        &type_names,
        &import_map,
        &mut entities,
    );

    // Extract TypeRef edges from function/method signatures
    extract_type_refs(
        source,
        &file_slug,
        lang,
        &function_names,
        &type_names,
        &import_map,
        &mut entities,
    );

    // Extract class constants
    variables::extract_variables(source, &file_slug, rel_path, "php", &mut entities);
    Ok(entities)
}

/// Extract function/method declarations from PHP source using kind-based matching.
///
/// Returns the set of function names found in this file (for call resolution).
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for kind_name in PHP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_php_func_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = is_php_public(&matched_text);
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            let cc = complexity::calculate(&matched_text, "php");
            let meta = metadata::extract_function_meta(&matched_text, "php");
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
                "php",
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

/// Extract type declarations from PHP source using kind-based matching.
///
/// Returns the set of type names found in this file (for TypeRef resolution).
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for kind_name in PHP_TYPE_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let type_kind = match *kind_name {
            "class_declaration" => "class",
            "interface_declaration" => "interface",
            "trait_declaration" => "trait_kind",
            "enum_declaration" => "enum_kind",
            _ => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let keyword = match type_kind {
                "class" => "class ",
                "interface" => "interface ",
                "trait_kind" => "trait ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("internal ");

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "php");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "php", type_meta.truncated,
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

/// Extract PHP `use` statements and produce Imports edges.
///
/// Resolves PSR-4 namespaces to file paths: `use Slim\Routing\RouteResolver;`
/// → `Slim/Routing/RouteResolver.php`. Only produces edges for imports whose
/// resolved path exists in `known_files`.
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

        // Match `use Namespace\Class;` or `use Namespace\Class as Alias;`
        if !trimmed.starts_with("use ") || trimmed.starts_with("use (") {
            continue;
        }

        // Strip `use ` prefix and `;` suffix
        let import_part = trimmed
            .strip_prefix("use ")
            .unwrap_or("")
            .trim_end_matches(';')
            .trim();

        if import_part.is_empty() || import_part.starts_with("function ") || import_part.starts_with("const ") {
            continue;
        }

        // Handle `use Ns\Class as Alias;`
        let (namespace_path, local_name) = if let Some(as_pos) = import_part.find(" as ") {
            let ns = import_part[..as_pos].trim();
            let alias = import_part[as_pos + 4..].trim();
            (ns, alias.to_string())
        } else {
            let name = import_part
                .rsplit('\\')
                .next()
                .unwrap_or(import_part)
                .to_string();
            (import_part, name)
        };

        // PSR-4: Convert namespace separators to path separators + .php
        let resolved_path = resolve_php_namespace(namespace_path);

        // Check if the resolved path exists in known_files
        let is_local = known_files.contains(&resolved_path);

        if is_local {
            let original_name = namespace_path
                .rsplit('\\')
                .next()
                .unwrap_or(namespace_path)
                .to_string();

            let target_slug = helpers::slugify_path(&resolved_path);
            import_map.insert(
                local_name,
                (target_slug.clone(), true, original_name),
            );

            edge_helpers::build_import_edge(
                file_slug,
                namespace_path,
                &resolved_path,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Resolve a PHP namespace to a file path using PSR-4 convention.
///
/// `Slim\Routing\RouteResolver` → `Slim/Routing/RouteResolver.php`
fn resolve_php_namespace(namespace: &str) -> String {
    let path = namespace.replace('\\', "/");
    format!("{path}.php")
}

/// Extract Calls edges from PHP function/method bodies.
///
/// For each function, finds:
/// - Bare function calls: `someFunction()`
/// - `$this->method()` calls (same-file method calls)
///
/// Resolves against local functions and import map.
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

    for kind_name in PHP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_php_func_name(&fn_text);
            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Extract function body (after first {)
            if let Some(body_start) = fn_text.find('{') {
                let body = &fn_text[body_start..];

                let mut callee_names = HashSet::new();

                // Bare function calls
                edge_helpers::extract_call_names_from_body(body, &mut callee_names);

                // $this->method() and self::method() calls
                edge_helpers::extract_member_call_names(body, &mut callee_names);

                // Resolve against local functions and imports
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

/// Extract TypeRef edges from PHP function/method signatures.
///
/// Parses type annotations in parameter types and return types.
/// PHP signatures: `function handle(AppRequest $request): AppResponse`
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

    for kind_name in PHP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_php_func_name(&fn_text);
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

            // Extract PHP-specific type annotations
            let mut type_names = HashSet::new();
            extract_php_type_annotations(signature, &mut type_names);

            // Resolve against local types and imports
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

/// Extract type names from PHP function signatures.
///
/// PHP type annotations appear:
/// - Before parameter names: `function handle(AppRequest $request)`
/// - After `:` for return types: `function handle(): AppResponse`
///
/// Filters out built-in PHP types (string, int, bool, array, etc.).
fn extract_php_type_annotations(signature: &str, out: &mut HashSet<String>) {
    let php_builtins: HashSet<&str> = [
        "string", "int", "integer", "float", "double", "bool", "boolean",
        "void", "null", "array", "object", "mixed", "never", "self", "static",
        "parent", "callable", "iterable", "resource", "true", "false",
    ]
    .into_iter()
    .collect();

    // Extract return type (after `):`  before `{`)
    if let Some(paren_close) = signature.rfind(')') {
        let after_params = &signature[paren_close + 1..];
        if let Some(colon_pos) = after_params.find(':') {
            let return_type = after_params[colon_pos + 1..].trim();
            // Handle nullable: ?TypeName
            let return_type = return_type.trim_start_matches('?');
            let type_name: String = return_type
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !type_name.is_empty() && !php_builtins.contains(type_name.as_str()) {
                out.insert(type_name);
            }
        }
    }

    // Extract parameter types (before $ signs in parameter list)
    if let Some(paren_open) = signature.find('(') {
        let paren_close = signature.rfind(')').unwrap_or(signature.len());
        let params_str = &signature[paren_open + 1..paren_close];

        for param in params_str.split(',') {
            let param = param.trim();
            // PHP param: `TypeName $varName` or `?TypeName $varName`
            if let Some(dollar_pos) = param.rfind('$') {
                let before_var = param[..dollar_pos].trim();
                // Handle nullable: ?TypeName
                let type_part = before_var.trim_end().trim_start_matches('?');
                let type_name: String = type_part
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !type_name.is_empty() && !php_builtins.contains(type_name.as_str()) {
                    out.insert(type_name);
                }
            }
        }
    }
}

/// Extract PHP function/method name: the identifier immediately after `function `.
fn extract_php_func_name(text: &str) -> String {
    helpers::extract_name_after_keyword(text, "function ")
}

/// Determine if a PHP function/method is public.
///
/// In PHP, methods without an explicit visibility modifier are public by default.
/// Only `private` and `protected` are non-public.
fn is_php_public(text: &str) -> bool {
    let trimmed = text.trim_start();
    !trimmed.starts_with("private ") && !trimmed.starts_with("protected ")
}
