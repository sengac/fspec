//! Java AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from Java source files
//! using kind-based AST matching.
//!
//! Uses `KindMatcher` to find `method_declaration` and type nodes rather than
//! pattern matching, which misses annotated methods (`@Override`, `@Test`),
//! constructors, and methods with generic return types.

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph_entities::GraphEntity;

/// AST node kinds for Java functions/methods.
const JAVA_FUNC_KINDS: &[&str] = &["method_declaration", "constructor_declaration"];

/// AST node kinds for Java type declarations.
const JAVA_TYPE_KINDS: &[(&str, &str)] = &[
    ("class_declaration", "class"),
    ("interface_declaration", "interface"),
    ("enum_declaration", "enum_kind"),
    ("record_declaration", "class"),
];

/// Extract entities from Java source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
pub fn extract_java(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Java;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test.java")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "java", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract method declarations → collect names for call resolution
    let function_names = extract_methods(&root, &file_slug, lang, &mut entities);

    // Extract type declarations → collect names for TypeRef resolution
    let type_names = extract_types(&root, &file_slug, lang, &mut entities);

    // Extract import statements → collect import map for cross-file resolution
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Extract Calls edges from method bodies
    extract_calls(source, &file_slug, lang, &function_names, &type_names, &import_map, &mut entities);

    // Extract TypeRef edges from method signatures
    extract_type_refs(
        source, &file_slug, lang, &function_names, &type_names, &import_map, &mut entities,
    );

    // Extract class-level variables
    variables::extract_variables(source, &file_slug, rel_path, "java", &mut entities);
    Ok(entities)
}

/// Extract method declarations from Java source using kind-based matching.
///
/// Returns the set of method names found in this file.
fn extract_methods(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for kind_name in JAVA_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_java_method_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = matched_text.contains("public ");
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            let cc = complexity::calculate(&matched_text, "java");
            let meta = metadata::extract_function_meta(&matched_text, "java");
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
                "java",
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

/// Extract type declarations from Java source using kind-based matching.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    lang: SupportLang,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (kind_name, type_kind) in JAVA_TYPE_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let keyword = match *type_kind {
                "class" => {
                    if matched_text.contains("record ") {
                        "record "
                    } else {
                        "class "
                    }
                }
                "interface" => "interface ",
                "enum_kind" => "enum ",
                _ => continue,
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = matched_text.contains("public ");

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "java");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "java", type_meta.truncated,
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

/// Extract Java `import` statements and produce Imports edges.
///
/// Resolves package paths to file paths: `com.myapp.service.UserService`
/// → `com/myapp/service/UserService.java`. Only produces edges for imports
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

        if !trimmed.starts_with("import ") || trimmed.starts_with("import static ") {
            continue;
        }

        let import_path = trimmed
            .strip_prefix("import ")
            .unwrap_or("")
            .trim_end_matches(';')
            .trim();

        if import_path.is_empty() {
            continue;
        }

        // Get local name (last segment)
        let local_name = import_path
            .rsplit('.')
            .next()
            .unwrap_or(import_path)
            .to_string();

        // Resolve to file path using suffix matching against known_files.
        // We resolve FIRST, then filter — this ensures project files like
        // `com/google/gson/Gson.java` produce Imports edges even though
        // "com.google" looks like an external prefix.
        let resolved_path = match resolve_java_import(import_path, known_files) {
            Some(p) => p,
            None => continue, // Not in known_files → skip (external/stdlib)
        };

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
    import_map
}

/// Resolve a Java package import to a file path using suffix matching.
///
/// `com.myapp.service.UserService` → looks for known file ending with
/// `com/myapp/service/UserService.java`. Falls back to exact match.
fn resolve_java_import(import_path: &str, known_files: &HashSet<String>) -> Option<String> {
    let suffix = import_path.replace('.', "/");
    let suffix_java = format!("{suffix}.java");

    // Try exact match first
    if known_files.contains(&suffix_java) {
        return Some(suffix_java);
    }

    // Try suffix match: find any known file ending with the package path
    let with_slash = format!("/{suffix_java}");
    if let Some(found) = known_files.iter().find(|f| f.ends_with(&with_slash)) {
        return Some(found.clone());
    }

    None
}

/// Extract Calls edges from Java method bodies.
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

    for kind_name in JAVA_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_java_method_name(&fn_text);
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

                // Extract `new ClassName(` patterns → TypeRef edges
                let mut constructor_names = HashSet::new();
                edge_helpers::extract_constructor_names_from_body(body, &mut constructor_names);
                for ctor_name in &constructor_names {
                    // Check local types first
                    if local_types.contains(ctor_name.as_str()) {
                        let target_slug = format!("{file_slug}::{ctor_name}");
                        edge_helpers::build_typeref_edge(&caller_slug, &target_slug, entities);
                    } else if let Some((target_file_slug, is_relative, original_name)) =
                        import_map.get(ctor_name.as_str())
                    {
                        if *is_relative {
                            let target_slug = format!("{target_file_slug}::{original_name}");
                            edge_helpers::build_typeref_edge(&caller_slug, &target_slug, entities);
                        }
                    }
                }
            }
        }
    }
}

/// Extract TypeRef edges from Java method signatures.
///
/// Java signatures: `public Response handle(Request req)`
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

    for kind_name in JAVA_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_java_method_name(&fn_text);
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
            extract_java_type_annotations(signature, &mut type_names);

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

/// Extract type names from Java method signatures.
///
/// Java type annotations appear:
/// - Return type: `public Response handle(...)` — word before method name
/// - Parameter types: `handle(Request req, String name)` — word before param name
///
/// Filters out Java primitives and common standard library types.
fn extract_java_type_annotations(signature: &str, out: &mut HashSet<String>) {
    let java_builtins: HashSet<&str> = [
        "void", "int", "long", "short", "byte", "float", "double",
        "boolean", "char", "String", "Object", "Integer", "Long",
        "Short", "Byte", "Float", "Double", "Boolean", "Character",
        "List", "Map", "Set", "Collection", "Optional", "Stream",
        "Iterable", "Iterator", "Comparable", "Serializable",
        "var", "final", "static", "public", "private", "protected",
        "abstract", "synchronized", "volatile", "transient", "native",
        "Override", "Test", "Deprecated",
    ]
    .into_iter()
    .collect();

    // Extract return type: word before method name (before `(`)
    if let Some(paren_pos) = signature.find('(') {
        let before_paren = signature[..paren_pos].trim();
        // Split by spaces: "public Response handle" → ["public", "Response", "handle"]
        let words: Vec<&str> = before_paren.split_whitespace().collect();
        if words.len() >= 2 {
            // Return type is the second-to-last word (last is method name)
            let return_type = words[words.len() - 2];
            let type_name: String = return_type
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !type_name.is_empty() && !java_builtins.contains(type_name.as_str()) {
                out.insert(type_name);
            }
        }
    }

    // Extract parameter types from parameter list
    if let Some(paren_open) = signature.find('(') {
        let paren_close = signature.rfind(')').unwrap_or(signature.len());
        let params_str = &signature[paren_open + 1..paren_close];

        for param in params_str.split(',') {
            let param = param.trim();
            // Java param: `TypeName varName` or `final TypeName varName`
            let words: Vec<&str> = param.split_whitespace().collect();
            if words.len() >= 2 {
                // Type is the word before the last word (var name)
                let type_word = words[words.len() - 2];
                let type_name: String = type_word
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !type_name.is_empty() && !java_builtins.contains(type_name.as_str()) {
                    out.insert(type_name);
                }
            }
        }
    }

    // Extract types from throws clause: `throws FooException, BarException`
    if let Some(throws_pos) = signature.find("throws ") {
        let after_throws = &signature[throws_pos + 7..];
        // Remove everything after '{' if present
        let throws_part = if let Some(brace) = after_throws.find('{') {
            &after_throws[..brace]
        } else {
            after_throws
        };
        for exception in throws_part.split(',') {
            let name: String = exception
                .trim()
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() && !java_builtins.contains(name.as_str()) {
                out.insert(name);
            }
        }
    }
}

/// Extract Java method name: the identifier immediately before `(`.
fn extract_java_method_name(text: &str) -> String {
    if let Some(paren_pos) = text.find('(') {
        let before = text[..paren_pos].trim();
        // Last word before (
        if let Some(last_space) = before.rfind(' ') {
            let name = &before[last_space + 1..];
            return name
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
        }
    }
    String::new()
}
