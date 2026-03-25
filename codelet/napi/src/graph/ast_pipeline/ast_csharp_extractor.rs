//! C# AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from C# source files.
//!
//! Uses `KindMatcher` for both methods and types. Resolves C# `using` statements
//! to file paths via namespace-to-path conversion.

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// AST node kinds for C# functions/methods.
const CSHARP_FUNC_KINDS: &[&str] = &["method_declaration", "constructor_declaration"];

/// AST node kinds for C# type declarations.
const CSHARP_TYPE_KINDS: &[(&str, &str)] = &[
    ("class_declaration", "class"),
    ("interface_declaration", "interface"),
    ("struct_declaration", "struct_kind"),
    ("enum_declaration", "enum_kind"),
    ("record_declaration", "class"),
];

/// C# system namespace prefixes that should NOT produce Imports edges.
const CSHARP_EXTERNAL_PREFIXES: &[&str] = &[
    "System", "Microsoft", "Newtonsoft", "NUnit", "Xunit",
];

/// Extract entities from C# source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
pub fn extract_csharp(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
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

    let root = lang.ast_grep(source);

    let function_names = extract_methods(&root, &file_slug, lang, &mut entities);
    let type_names = extract_types(&root, &file_slug, lang, &mut entities);
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    extract_calls(source, &file_slug, lang, &function_names, &import_map, &mut entities);
    extract_type_refs(
        source, &file_slug, lang, &function_names, &type_names, &import_map, &mut entities,
    );

    Ok(entities)
}

/// Extract method declarations from C# source using kind-based matching.
fn extract_methods(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for kind_name in CSHARP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_csharp_method_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = matched_text.contains("public ");
            let is_async = matched_text.contains("async ");
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug, &name, is_async, is_public, param_count,
                start_pos.line() as i32 + 1, end_pos.line() as i32 + 1,
            ));
            entities.push(helpers::build_contains_edge(file_slug, &fn_slug, "Contains"));
        }
    }
    seen_names
}

/// Extract type declarations from C# source using kind-based matching.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (kind_name, type_kind) in CSHARP_TYPE_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => if matched_text.contains("record ") { "record " } else { "class " },
                "interface" => "interface ",
                "struct_kind" => "struct ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = matched_text.contains("public ");
            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(file_slug, &name, type_kind, is_public));
            entities.push(helpers::build_contains_edge(file_slug, &type_slug, "ContainsType"));
        }
    }
    seen_names
}

/// Extract C# `using` statements and produce Imports edges.
fn extract_imports(
    source: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("using ") || trimmed.starts_with("using static ") {
            continue;
        }
        // Skip `using (var x = ...)` resource pattern
        if trimmed.starts_with("using (") {
            continue;
        }

        let ns_path = trimmed
            .strip_prefix("using ")
            .unwrap_or("")
            .trim_end_matches(';')
            .trim();

        if ns_path.is_empty() {
            continue;
        }

        // Skip system/external namespaces
        if CSHARP_EXTERNAL_PREFIXES.iter().any(|p| ns_path.starts_with(p)) {
            continue;
        }

        let local_name = ns_path.rsplit('.').next().unwrap_or(ns_path).to_string();
        let resolved_path = format!("{}.cs", ns_path.replace('.', "/"));
        let is_local = known_files.contains(&resolved_path);

        if is_local {
            let target_slug = helpers::slugify_path(&resolved_path);
            import_map.insert(local_name.clone(), (target_slug, true, local_name));
            edge_helpers::build_import_edge(file_slug, ns_path, &resolved_path, false, entities);
        }
    }
    import_map
}

/// Extract Calls edges from C# method bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    lang: SupportLang,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let root = lang.ast_grep(source);
    for kind_name in CSHARP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_csharp_method_name(&fn_text);
            if fn_name.is_empty() { continue; }
            let caller_slug = format!("{file_slug}::{fn_name}");
            if let Some(body_start) = fn_text.find('{') {
                let body = &fn_text[body_start..];
                let mut callee_names = HashSet::new();
                edge_helpers::extract_call_names_from_body(body, &mut callee_names);
                edge_helpers::resolve_calls(
                    &caller_slug, file_slug, &callee_names, &fn_name,
                    local_functions, import_map, entities,
                );
            }
        }
    }
}

/// Extract TypeRef edges from C# method signatures.
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
    for kind_name in CSHARP_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };
        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_csharp_method_name(&fn_text);
            if fn_name.is_empty() || !function_names.contains(&fn_name) { continue; }
            let fn_slug = format!("{file_slug}::{fn_name}");
            let signature = fn_text.split('{').next().unwrap_or(&fn_text);
            let mut type_names = HashSet::new();
            extract_csharp_type_annotations(signature, &mut type_names);
            edge_helpers::resolve_type_refs(
                &fn_slug, file_slug, &type_names, local_types, import_map, entities,
            );
        }
    }
}

/// Extract type names from C# method signatures (same pattern as Java).
fn extract_csharp_type_annotations(signature: &str, out: &mut HashSet<String>) {
    let builtins: HashSet<&str> = [
        "void", "int", "long", "short", "byte", "float", "double",
        "bool", "char", "string", "object", "decimal", "dynamic",
        "var", "String", "Int32", "Int64", "Boolean", "Object",
        "Task", "Action", "Func", "IEnumerable", "IList", "IDictionary",
        "List", "Dictionary", "HashSet", "IDisposable",
        "public", "private", "protected", "internal", "static",
        "async", "override", "virtual", "abstract", "sealed", "new",
    ].into_iter().collect();

    // Return type: word before method name
    if let Some(paren_pos) = signature.find('(') {
        let before = signature[..paren_pos].trim();
        let words: Vec<&str> = before.split_whitespace().collect();
        if words.len() >= 2 {
            let return_type = words[words.len() - 2];
            let name: String = return_type.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            if !name.is_empty() && !builtins.contains(name.as_str()) {
                out.insert(name);
            }
        }
    }

    // Parameter types
    if let Some(paren_open) = signature.find('(') {
        let paren_close = signature.rfind(')').unwrap_or(signature.len());
        let params_str = &signature[paren_open + 1..paren_close];
        for param in params_str.split(',') {
            let words: Vec<&str> = param.trim().split_whitespace().collect();
            if words.len() >= 2 {
                let type_word = words[words.len() - 2];
                let name: String = type_word.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                if !name.is_empty() && !builtins.contains(name.as_str()) {
                    out.insert(name);
                }
            }
        }
    }
}

/// Extract C# method name: the identifier immediately before `(`.
fn extract_csharp_method_name(text: &str) -> String {
    if let Some(paren_pos) = text.find('(') {
        let before = text[..paren_pos].trim();
        if let Some(last_space) = before.rfind(' ') {
            let name = &before[last_space + 1..];
            return name.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        }
    }
    String::new()
}
