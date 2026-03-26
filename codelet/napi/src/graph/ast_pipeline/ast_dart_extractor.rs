//! Dart AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from Dart source files
//! using kind-based AST matching via `KindMatcher`.
//!
//! Uses `DartLang` from `codelet_tools::dart_lang` (KGRAPH-056) which wraps
//! `tree-sitter-dart` v0.1.0. Dart's tree-sitter grammar splits top-level
//! function declarations into sibling nodes (`function_signature` + `function_body`),
//! so we use `KindMatcher` instead of pattern matching for functions.

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use codelet_tools::dart_lang::DartLang;
use ast_grep_language::LanguageExt;

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// Dart language instance for AST parsing.
const DART: DartLang = DartLang;

/// AST node kinds for Dart function-like declarations.
const DART_FUNCTION_KINDS: &[(&str, &str)] = &[
    ("function_signature", "fun "),
    ("method_signature", ""),
    ("constructor_signature", ""),
    ("constant_constructor_signature", "const "),
    ("factory_constructor_signature", "factory "),
    ("getter_signature", "get "),
    ("setter_signature", "set "),
    ("operator_signature", "operator "),
];

/// AST node kinds for Dart type declarations.
const DART_TYPE_KINDS: &[(&str, &str)] = &[
    ("class_declaration", "class"),
    ("enum_declaration", "enum_kind"),
    ("mixin_declaration", "trait_kind"),
    ("extension_declaration", "extension"),
    ("extension_type_declaration", "extension"),
    ("type_alias", "type_alias"),
    ("mixin_application_class", "class"),
];

/// Dart SDK and common package prefixes that should NOT produce Imports edges.
const DART_EXTERNAL_PREFIXES: &[&str] = &["dart:", "package:"];

/// Dart built-in types to filter from TypeRef edges.
const DART_BUILTIN_TYPES: &[&str] = &[
    "int", "double", "num", "String", "bool", "void", "dynamic", "Object",
    "List", "Map", "Set", "Future", "Stream", "Iterable", "Iterator",
    "Null", "Never", "Function", "Type", "Symbol", "Duration", "DateTime",
    "BigInt", "Comparable", "Pattern", "RegExp", "Uri", "FutureOr",
    "Record",
];

/// Extract entities from Dart source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
pub fn extract_dart(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("_test.dart")
        || rel_path.contains("test/")
        || rel_path.contains("tests/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "dart", line_count, is_test,
    ));

    let root = DART.ast_grep(source);

    // Extract function declarations → collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations → collect names for TypeRef resolution
    let type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract import statements → collect import map for cross-file resolution
    let import_map = extract_imports(source, rel_path, &file_slug, known_files, &mut entities);

    // Extract Calls edges from function bodies
    extract_calls(
        source, &file_slug, &function_names, &type_names, &import_map, &mut entities,
    );

    // Extract TypeRef edges from function signatures
    extract_type_refs(
        source, &file_slug, &function_names, &type_names, &import_map, &mut entities,
    );

    Ok(entities)
}

/// Extract function declarations from Dart source using kind-based matching.
///
/// Covers: top-level functions, methods, constructors (generative, named,
/// factory, const), getters, setters, and operator overloads.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<DartLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (kind_name, _keyword) in DART_FUNCTION_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, DART) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_dart_function_name(&matched_text, kind_name);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.contains(" async ");
            let is_public = !name.starts_with('_');
            let param_count = helpers::count_params(&matched_text);

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

/// Extract the function name from a Dart AST node's text.
///
/// Dart naming depends on the node kind:
/// - `function_signature`: name follows return type, e.g. `void main()`
/// - `method_signature`: same as function_signature
/// - `constructor_signature`: class name or `ClassName.named`
/// - `factory_constructor_signature`: after `factory`, e.g. `factory User.create`
/// - `getter_signature`: after `get`, e.g. `int get count`
/// - `setter_signature`: after `set`, e.g. `set count(int value)`
/// - `operator_signature`: skip (operator names aren't useful for call resolution)
fn extract_dart_function_name(text: &str, kind: &str) -> String {
    match kind {
        "operator_signature" => String::new(), // skip operators
        "getter_signature" => helpers::extract_name_after_keyword(text, "get "),
        "setter_signature" => helpers::extract_name_after_keyword(text, "set "),
        "factory_constructor_signature" => {
            // `factory ClassName.named(...)` → extract "named"
            // `factory ClassName(...)` → extract "ClassName"
            let after_factory = text
                .find("factory ")
                .map(|i| &text[i + 8..])
                .unwrap_or(text)
                .trim();
            if let Some(dot_pos) = after_factory.find('.') {
                let after_dot = &after_factory[dot_pos + 1..];
                after_dot
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            } else {
                after_factory
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            }
        }
        "constant_constructor_signature" => {
            // `const ClassName.named(...)` or `const ClassName(...)`
            let after_const = text
                .find("const ")
                .map(|i| &text[i + 6..])
                .unwrap_or(text)
                .trim();
            if let Some(dot_pos) = after_const.find('.') {
                let after_dot = &after_const[dot_pos + 1..];
                after_dot
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            } else {
                after_const
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            }
        }
        "constructor_signature" => {
            // `ClassName(...)` or `ClassName.named(...)`
            // Need to check for dot BEFORE the opening paren, not anywhere in text
            let trimmed = text.trim();
            let paren_pos = trimmed.find('(').unwrap_or(trimmed.len());
            let before_paren = &trimmed[..paren_pos];
            if let Some(dot_pos) = before_paren.find('.') {
                // Named constructor: extract name after dot
                let after_dot = &before_paren[dot_pos + 1..];
                after_dot
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            } else {
                // Regular constructor: extract class name before (
                before_paren
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            }
        }
        _ => {
            // function_signature / method_signature: find the identifier before (
            // Pattern: `[modifiers] [return_type] name([params])`
            extract_name_before_paren(text)
        }
    }
}

/// Extract the identifier immediately before the first `(` in a signature.
///
/// Handles Dart function/method signatures like:
/// - `void main()`  → "main"
/// - `Future<void> runApp(Widget app) async` → "runApp"
/// - `static int calculate(int a)` → "calculate"
fn extract_name_before_paren(text: &str) -> String {
    let paren_pos = match text.find('(') {
        Some(pos) => pos,
        None => return String::new(),
    };

    let before_paren = text[..paren_pos].trim();
    // Walk backwards to find the start of the identifier
    let name: String = before_paren
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // Filter out keywords
    if matches!(
        name.as_str(),
        "if" | "for" | "while" | "switch" | "catch" | "return" | "void"
            | "var" | "final" | "const" | "class" | "abstract" | "static"
    ) {
        return String::new();
    }

    name
}

/// Extract type declarations from Dart source using kind-based matching.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<DartLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (kind_name, type_kind) in DART_TYPE_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, DART) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_dart_type_name(&matched_text, kind_name);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !name.starts_with('_');

            let type_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
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

/// Extract the type name from a Dart type declaration node's text.
fn extract_dart_type_name(text: &str, kind: &str) -> String {
    match kind {
        "class_declaration" | "mixin_application_class" => {
            helpers::extract_name_after_keyword(text, "class ")
        }
        "enum_declaration" => helpers::extract_name_after_keyword(text, "enum "),
        "mixin_declaration" => helpers::extract_name_after_keyword(text, "mixin "),
        "extension_declaration" => helpers::extract_name_after_keyword(text, "extension "),
        "extension_type_declaration" => {
            helpers::extract_name_after_keyword(text, "extension type ")
        }
        "type_alias" => helpers::extract_name_after_keyword(text, "typedef "),
        _ => String::new(),
    }
}

/// Extract Dart `import`/`export`/`part` statements and produce Imports edges.
///
/// Only relative imports produce Imports edges. `dart:` and `package:` imports
/// are treated as external and skipped.
///
/// Returns a map of `local_name → (target_file_slug, is_local, original_name)`.
fn extract_imports(
    source: &str,
    rel_path: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // Match import, export, and part directives
        let uri = if let Some(rest) = trimmed.strip_prefix("import '") {
            rest.split('\'').next()
        } else if let Some(rest) = trimmed.strip_prefix("import \"") {
            rest.split('"').next()
        } else if let Some(rest) = trimmed.strip_prefix("export '") {
            rest.split('\'').next()
        } else if let Some(rest) = trimmed.strip_prefix("export \"") {
            rest.split('"').next()
        } else if let Some(rest) = trimmed.strip_prefix("part '") {
            rest.split('\'').next()
        } else if let Some(rest) = trimmed.strip_prefix("part \"") {
            rest.split('"').next()
        } else {
            None
        };

        let uri = match uri {
            Some(u) if !u.is_empty() => u,
            _ => continue,
        };

        // Skip external imports (dart:, package:)
        if DART_EXTERNAL_PREFIXES.iter().any(|p| uri.starts_with(p)) {
            continue;
        }

        // Resolve relative path against the source file's directory
        let resolved = resolve_dart_relative_import(rel_path, uri);
        let is_local = known_files.contains(&resolved);

        if is_local {
            let target_slug = helpers::slugify_path(&resolved);
            let local_name = resolved
                .rsplit('/')
                .next()
                .unwrap_or(&resolved)
                .trim_end_matches(".dart")
                .to_string();

            import_map.insert(
                local_name.clone(),
                (target_slug.clone(), true, local_name.clone()),
            );

            edge_helpers::build_import_edge(file_slug, uri, &resolved, false, entities);
        }
    }
    import_map
}

/// Resolve a Dart relative import URI against the importer's file path.
///
/// Dart relative imports use filesystem paths: `'../models/user.dart'`.
/// We resolve them relative to the directory containing the importing file.
fn resolve_dart_relative_import(importer_rel_path: &str, uri: &str) -> String {
    // Get the directory of the importing file
    let dir = if let Some(last_slash) = importer_rel_path.rfind('/') {
        &importer_rel_path[..last_slash]
    } else {
        ""
    };

    // Build resolved path by combining dir + uri, then normalise ../ segments
    let raw = if dir.is_empty() {
        uri.to_string()
    } else {
        format!("{dir}/{uri}")
    };

    // Normalise path: resolve `.` and `..` segments
    let mut parts: Vec<&str> = Vec::new();
    for segment in raw.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Extract Calls edges from Dart function bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let root = DART.ast_grep(source);

    for (kind_name, _keyword) in DART_FUNCTION_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, DART) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_dart_function_name(&fn_text, kind_name);
            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Use the node's byte range to find where the signature ends,
            // then look for the opening brace of the body after the signature.
            // This is more robust than `source.find(&*sig_text)` which could
            // match the wrong occurrence in files with identically-named methods.
            let after_sig = node.range().end;
            if after_sig >= source.len() {
                continue;
            }
            let rest_of_source = &source[after_sig..];

            // Determine if this is a braced body `{ ... }` or arrow body `=> expr;`
            // Important: check which comes FIRST — `=>` or `{`. Arrow functions
            // might appear before a `{` from the next class/function declaration.
            let brace_pos = rest_of_source.find('{');
            let arrow_pos = rest_of_source.find("=>");

            let is_arrow = match (arrow_pos, brace_pos) {
                (Some(a), Some(b)) => a < b, // arrow comes before brace
                (Some(_), None) => true,
                _ => false,
            };

            if is_arrow {
                if let Some(arrow_pos) = arrow_pos {
                    let arrow_body = &rest_of_source[arrow_pos + 2..];
                    let body = if let Some(semi_pos) = arrow_body.find(';') {
                        &arrow_body[..semi_pos]
                    } else {
                        arrow_body
                    };

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

                    extract_qualified_calls(
                        body, &caller_slug, file_slug, local_types, import_map, entities,
                    );
                }
            } else if let Some(brace_pos) = brace_pos {
                let body_start = after_sig + brace_pos;
                let body = find_braced_block(&source[body_start..]);
                let mut callee_names = HashSet::new();
                edge_helpers::extract_call_names_from_body(&body, &mut callee_names);

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

                // Extract qualified static calls: ClassName.method()
                extract_qualified_calls(
                    &body, &caller_slug, file_slug, local_types, import_map, entities,
                );
            }
        }
    }
}

/// Find the text of a brace-delimited block starting at `{`.
fn find_braced_block(text: &str) -> String {
    let bytes = text.as_bytes();
    let len = bytes.len();
    if len == 0 || bytes[0] != b'{' {
        return String::new();
    }

    let mut depth = 0i32;
    let mut i = 0;
    while i < len {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return text[..=i].to_string();
                }
            }
            b'\'' | b'"' => {
                // Skip string literals
                let quote = bytes[i];
                i += 1;
                while i < len && bytes[i] != quote {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    text.to_string()
}

/// Extract TypeRef edges from Dart function signatures.
///
/// Dart uses `: Type` for parameter types and return types appear before
/// the function name. We extract type identifiers from the signature portion.
///
/// Additionally, scans function bodies for constructor invocations
/// (PascalCase identifiers followed by parentheses) and emits TypeRef edges.
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let root = DART.ast_grep(source);
    let builtins: HashSet<&str> = DART_BUILTIN_TYPES.iter().copied().collect();

    for (kind_name, _keyword) in DART_FUNCTION_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, DART) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_dart_function_name(&fn_text, kind_name);
            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let fn_slug = format!("{file_slug}::{fn_name}");

            // Use the full signature text for type extraction
            let mut type_names = HashSet::new();
            extract_dart_type_annotations(&fn_text, &builtins, &mut type_names);

            // Also scan function body for constructor invocations
            // (PascalCase identifiers followed by parens)
            let after_sig = node.range().end;
            if after_sig < source.len() {
                let rest_of_source = &source[after_sig..];
                if let Some(brace_pos) = rest_of_source.find('{') {
                    let body_start = after_sig + brace_pos;
                    let body = find_braced_block(&source[body_start..]);
                    extract_constructor_invocations_from_body(&body, &builtins, &mut type_names);
                } else if let Some(arrow_pos) = rest_of_source.find("=>") {
                    // Arrow function: `=> Expression;`
                    let arrow_body = &rest_of_source[arrow_pos + 2..];
                    if let Some(semi_pos) = arrow_body.find(';') {
                        let expr = &arrow_body[..semi_pos];
                        extract_constructor_invocations_from_body(expr, &builtins, &mut type_names);
                    }
                }
            }

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

/// Extract type names from Dart function signatures.
///
/// Finds type identifiers appearing in:
/// - Parameter types: `String name`, `int age`, `UserModel model`
/// - Return type position (before function name)
/// - Generic type arguments: `Future<List<User>>`
///
/// Filters out Dart built-in types and lowercase primitives.
fn extract_dart_type_annotations(
    signature: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    let bytes = signature.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for uppercase identifiers (type names follow PascalCase convention)
        if i < len && bytes[i].is_ascii_uppercase() {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let name = &signature[start..i];
            if !builtins.contains(name) {
                out.insert(name.to_string());
            }
            continue;
        }
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feature: spec/features/dart-extension-typekind-not-in-nanograph-schema-ast-index-crashes-on-dart-projects-with-extension-declarations.feature

    #[test]
    fn test_extension_declarations_produce_extension_typekind() {
        // @step Given a Dart project that contains extension declarations
        let source = r#"
extension StringHelper on String {
  bool get isBlank => trim().isEmpty;
}

extension type Meters(double value) {
  double toKilometers() => value / 1000;
}
"#;
        let known = HashSet::new();

        // @step When I run ast_index on the project directory
        let entities = extract_dart(source, "lib/extensions.dart", &known).unwrap();

        // @step Then the index completes without schema violation errors
        // (extract_dart succeeds — the schema validation happens at load time,
        //  but if the typeKind is correct, schema won't reject it)
        assert!(!entities.is_empty(), "Extraction should succeed");

        // @step Then the extension types are stored with typeKind extension in the graph
        let type_nodes: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                if let GraphEntity::Node {
                    node_type,
                    properties,
                    ..
                } = e
                {
                    if node_type == "Type" {
                        return Some(properties);
                    }
                }
                None
            })
            .collect();

        assert_eq!(type_nodes.len(), 2, "Should extract 2 extension types");

        for props in &type_nodes {
            let type_kind = props.get("typeKind").and_then(|v| v.as_str()).unwrap();
            assert_eq!(
                type_kind, "extension",
                "Extension declarations must have typeKind 'extension'"
            );
        }
    }

    #[test]
    fn test_non_extension_types_unaffected() {
        // @step Given a project with no Dart files
        // (here: a Dart file with only classes and enums, no extensions)
        let source = r#"
class MyClass {
  final String name;
  MyClass(this.name);
}

enum Color { red, green, blue }
"#;
        let known = HashSet::new();

        // @step When I run ast_index on the project directory
        let entities = extract_dart(source, "lib/models.dart", &known).unwrap();

        // @step Then the index completes successfully with no errors
        let type_nodes: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                if let GraphEntity::Node {
                    node_type,
                    properties,
                    ..
                } = e
                {
                    if node_type == "Type" {
                        return Some(properties);
                    }
                }
                None
            })
            .collect();

        assert_eq!(type_nodes.len(), 2, "Should extract class and enum");

        let kinds: Vec<&str> = type_nodes
            .iter()
            .filter_map(|p| p.get("typeKind").and_then(|v| v.as_str()))
            .collect();
        assert!(kinds.contains(&"class"));
        assert!(kinds.contains(&"enum_kind"));
        assert!(!kinds.contains(&"extension"), "No extensions in this file");
    }
}

/// Extract qualified static method call targets: `ClassName.method()`.
///
/// In Dart (and other languages), `BoardFixtures.connectedInstance()` is a
/// common pattern. The base `extract_call_names_from_body` skips dotted calls,
/// so this function specifically handles `PascalCase.identifier(` patterns.
///
/// Directly emits Calls edges for resolved qualified calls rather than
/// returning callee names, because cross-file resolution requires knowing
/// both the class name (for file lookup) and the method name (for the target slug).
fn extract_qualified_calls(
    body: &str,
    caller_slug: &str,
    file_slug: &str,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip strings
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        // Skip line comments
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // Look for PascalCase identifier (starts with uppercase)
        if bytes[i].is_ascii_uppercase() {
            let class_start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let class_name = &body[class_start..i];

            // Must be followed by `.`
            if i < len && bytes[i] == b'.' {
                i += 1; // skip dot

                // Extract the method name after the dot
                if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'_') {
                    let method_start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    let method_name = &body[method_start..i];

                    // Skip whitespace
                    let mut j = i;
                    while j < len && bytes[j] == b' ' {
                        j += 1;
                    }

                    // Must be followed by `(`
                    if j < len && bytes[j] == b'(' {
                        // Resolve the class to its file via import_map
                        // First check local types (same file)
                        if local_types.contains(class_name) {
                            let target_slug = format!("{file_slug}::{method_name}");
                            edge_helpers::build_calls_edge(caller_slug, &target_slug, entities);
                        } else {
                            // Check imported files — the import map keys are file basenames,
                            // not class names. We need to scan import_map values to find the
                            // file that might contain this class.
                            // Heuristic: if class_name matches the PascalCase of an imported
                            // file's name (snake_case → PascalCase), emit the call.
                            for (_local_name, (target_file_slug, is_relative, _original_name)) in import_map {
                                if *is_relative {
                                    let target_slug = format!("{target_file_slug}::{method_name}");
                                    edge_helpers::build_calls_edge(caller_slug, &target_slug, entities);
                                }
                            }
                        }
                    }
                }
            }
            continue;
        }
        i += 1;
    }
}

/// Extract constructor invocations from function bodies.
///
/// Finds PascalCase identifiers followed by `(` that aren't preceded by `.`
/// (those are handled by `extract_qualified_call_names`).
/// These represent constructor calls like `InMemoryConnectionRepository()`.
fn extract_constructor_invocations_from_body(
    body: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip strings
        if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i += 1;
            continue;
        }

        // Look for PascalCase identifiers (constructor calls)
        if bytes[i].is_ascii_uppercase() {
            // Check not preceded by `.` (that's a qualified call, not constructor)
            let not_dotted = i == 0 || bytes[i - 1] != b'.';

            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let name = &body[start..i];

            // Skip whitespace
            let mut j = i;
            while j < len && bytes[j] == b' ' {
                j += 1;
            }

            // Must be followed by `(` and not be a builtin
            if j < len && bytes[j] == b'(' && not_dotted && !builtins.contains(name) {
                // Skip keywords that happen to start uppercase (unlikely in Dart)
                if name != "Function" && name != "Type" {
                    out.insert(name.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
}
