//! TypeScript/JavaScript AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from TypeScript/JavaScript
//! source files using ast-grep pattern matching.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};
use serde_json::{Map, Value};

use super::complexity;
use super::metadata;
use super::variables;
use super::helpers;
use crate::graph_entities::GraphEntity;

/// ast-grep patterns for TypeScript function declarations.
///
/// Two patterns cover all cases:
/// - Functions with a return type annotation: `function name(args): RetType { body }`
/// - Functions without a return type: `function name(args) { body }`
///
/// Both patterns match async functions (async is a modifier on the AST node).
/// Export status is determined by checking the parent node text.
const TS_FUNCTION_PATTERNS: &[&str] = &[
    "function $NAME($$$ARGS): $RET { $$$BODY }",
    "function $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep pattern for TypeScript import statements.
const TS_IMPORT_PATTERN: &str = "import $$$IMPORTS from $SOURCE";

/// ast-grep patterns for TypeScript type declarations.
const TS_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("interface $NAME { $$$BODY }", "interface"),
    ("type $NAME = $$$DEF", "type_alias"),
    ("enum $NAME { $$$BODY }", "enum_kind"),
    ("class $NAME { $$$BODY }", "class"),
];

/// Extract entities from TypeScript/JavaScript source code.
///
/// The `known_files` set contains all relative paths of source files in the
/// project. When non-empty, it enables barrel-import resolution: an import
/// like `from '../types'` that resolves to `src/types.ts` will be corrected
/// to `src/types/index.ts` if the latter exists in `known_files` but the
/// former does not. Pass an empty set when extracting a single file in
/// isolation (barrel resolution is best-effort in that case).
pub fn extract_typescript(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::TypeScript;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    // Create File node
    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("test")
        || rel_path.contains("spec")
        || rel_path.contains("__tests__");
    let language = if rel_path.ends_with(".tsx") || rel_path.ends_with(".jsx") {
        "tsx"
    } else {
        "typescript"
    };

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, language, line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations — collect names for call resolution
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations (interfaces, type aliases, enums, classes)
    let type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract import statements — collect imported name→target slug mappings
    let import_map = extract_imports(&root, &file_slug, rel_path, known_files, &mut entities);

    // Extract Calls edges by scanning function bodies for call expressions
    extract_calls(&root, &file_slug, &function_names, &import_map, &mut entities);

    // Extract TypeRef edges by scanning function signatures for type annotations
    extract_type_refs(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

    // Extract module-level and class-level variables
    variables::extract_variables(source, &file_slug, rel_path, language, &mut entities);

    Ok(entities)
}

/// Extract function declarations from TypeScript source using multiple patterns.
///
/// Returns the set of function names found in this file (for call resolution).
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for pattern in TS_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_function_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_async = matched_text.contains("async ");
            let is_public = node
                .parent()
                .is_some_and(|p| p.text().starts_with("export "));
            let param_count = helpers::count_params(&matched_text);
            let cc = complexity::calculate(&matched_text, "typescript");
            let meta = metadata::extract_function_meta(&matched_text, "typescript");

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
                "typescript",
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

/// Extract type declarations (interface, type alias, enum, class) from TypeScript source.
///
/// Returns the set of type names found in this file (for TypeRef resolution).
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in TS_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = extract_type_name(&matched_text, type_kind);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = node
                .parent()
                .is_some_and(|p| p.text().starts_with("export "));

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "typescript");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "typescript", type_meta.truncated,
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

/// Extract import statements from TypeScript source.
///
/// Returns a map of imported identifier name → (target file slug, is_relative),
/// used for resolving cross-file Calls and TypeRef edges.
/// Only relative imports (from `./` or `../`) produce Calls/TypeRef edges.
///
/// Returns a map of `local_name -> (target_file_slug, is_relative, original_name)`.
/// The `original_name` is the exported name in the target file (before `as` alias).
fn extract_imports(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for node in root.root().find_all(TS_IMPORT_PATTERN) {
        let matched_text = node.text();
        let import_path = extract_import_path(&matched_text);
        if import_path.is_empty() {
            continue;
        }

        let is_relative = import_path.starts_with('.');

        // Resolve relative import to a file path
        let resolved = resolve_import_path(rel_path, &import_path, known_files);
        let target_slug = helpers::slugify_path(&resolved);

        // Extract imported identifiers as (local_name, original_name) pairs
        let imported_names = extract_imported_names(&matched_text);
        for (local_name, original_name) in &imported_names {
            import_map.insert(
                local_name.clone(),
                (target_slug.clone(), is_relative, original_name.clone()),
            );
        }

        // Create target File node (may be merged if already exists in graph)
        let mut target_props = Map::new();
        target_props.insert("slug".to_string(), Value::String(target_slug.clone()));
        target_props.insert("path".to_string(), Value::String(resolved.clone()));
        entities.push(GraphEntity::Node {
            node_type: "File".to_string(),
            slug: target_slug.clone(),
            properties: target_props,
        });

        // Imports edge
        let is_type_only = matched_text.contains("import type ");
        let mut edge_props = Map::new();
        edge_props.insert(
            "importPath".to_string(),
            Value::String(import_path.to_string()),
        );
        edge_props.insert("isTypeOnly".to_string(), Value::Bool(is_type_only));
        entities.push(GraphEntity::Edge {
            edge_type: "Imports".to_string(),
            from_slug: file_slug.to_string(),
            to_slug: target_slug,
            properties: edge_props,
        });
    }
    import_map
}

/// Extract Calls edges by scanning function bodies for call expressions.
///
/// For each function in the file, finds bare identifier calls (not method calls)
/// and resolves them to known functions in the same file or imported functions.
fn extract_calls(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    local_functions: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    // For each function, scan its body for call expressions
    for pattern in TS_FUNCTION_PATTERNS {
        for fn_node in root.root().find_all(*pattern) {
            let fn_text = fn_node.text();
            let fn_name = extract_function_name(&fn_text);
            if fn_name.is_empty() {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Find the body: everything between first { and last }
            if let Some(body_start) = fn_text.find('{') {
                let body = &fn_text[body_start..];
                // Find all identifier( patterns — bare function calls
                let mut seen_callees = HashSet::new();
                extract_call_names_from_body(body, &mut seen_callees);

                for callee_name in &seen_callees {
                    // Skip self-calls
                    if callee_name == &fn_name {
                        continue;
                    }

                    // Resolve: same-file function?
                    if local_functions.contains(callee_name.as_str()) {
                        let target_slug = format!("{file_slug}::{callee_name}");
                        entities.push(GraphEntity::Edge {
                            edge_type: "Calls".to_string(),
                            from_slug: caller_slug.clone(),
                            to_slug: target_slug,
                            properties: Map::new(),
                        });
                    } else if let Some((target_file_slug, is_relative, original_name)) =
                        import_map.get(callee_name.as_str())
                    {
                        // Cross-file call via import — only for relative imports
                        // (external packages don't have Function nodes in our graph)
                        // Use original_name (the exported name) not the local alias
                        if *is_relative {
                            let target_slug = format!("{target_file_slug}::{original_name}");
                            entities.push(GraphEntity::Edge {
                                edge_type: "Calls".to_string(),
                                from_slug: caller_slug.clone(),
                                to_slug: target_slug,
                                properties: Map::new(),
                            });
                        }
                    }
                    // If not found in local or imports, it's a builtin/unknown — skip
                }
            }
        }
    }
}

/// Extract bare function call names from a function body string.
///
/// Finds patterns like `identifier(` but NOT `something.identifier(` (method calls).
/// Also excludes `new Identifier(` (constructor calls).
fn extract_call_names_from_body(body: &str, out: &mut HashSet<String>) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Skip strings
        if bytes[i] == b'\'' || bytes[i] == b'"' || bytes[i] == b'`' {
            let quote = bytes[i];
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' {
                    i += 1; // skip escaped char
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

        // Skip block comments
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        }

        // Look for identifier followed by (
        if is_ident_start(bytes[i]) {
            let start = i;
            while i < len && is_ident_char(bytes[i]) {
                i += 1;
            }
            let name = &body[start..i];

            // Skip whitespace between identifier and (
            let mut j = i;
            while j < len && bytes[j] == b' ' {
                j += 1;
            }

            if j < len && bytes[j] == b'(' {
                // Check: not preceded by `.` (method call) or `new ` (constructor)
                let not_method = start == 0 || bytes[start - 1] != b'.';
                let not_constructor = if start >= 4 {
                    // Use .get() for safe slicing — start-4 may land inside
                    // a multi-byte UTF-8 char (e.g., box-drawing '─' is 3 bytes).
                    body.get(start - 4..start) != Some("new ")
                } else {
                    true
                };
                // Not a keyword
                let not_keyword = !matches!(
                    name,
                    "if" | "for" | "while" | "switch" | "catch" | "return"
                        | "typeof" | "instanceof" | "await" | "function"
                );

                if not_method && not_constructor && not_keyword {
                    out.insert(name.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Extract TypeRef edges by scanning function signatures for type annotations.
///
/// Looks for `: TypeName` patterns in function parameters and return types,
/// and resolves them against known types in the same file or imported types.
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::TypeScript;
    let root = lang.ast_grep(source);

    for pattern in TS_FUNCTION_PATTERNS {
        for fn_node in root.root().find_all(*pattern) {
            let fn_text = fn_node.text();
            let fn_name = extract_function_name(&fn_text);
            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let caller_slug = format!("{file_slug}::{fn_name}");

            // Extract signature: everything before the first {
            let signature = if let Some(brace_pos) = fn_text.find('{') {
                &fn_text[..brace_pos]
            } else {
                &fn_text
            };

            // Find type annotations: `: TypeName` patterns
            let mut seen_types = HashSet::new();
            extract_type_names_from_signature(signature, &mut seen_types);

            for type_name in &seen_types {
                // Resolve: same-file type?
                if local_types.contains(type_name.as_str()) {
                    let target_slug = format!("{file_slug}::{type_name}");
                    entities.push(GraphEntity::Edge {
                        edge_type: "TypeRef".to_string(),
                        from_slug: caller_slug.clone(),
                        to_slug: target_slug,
                        properties: Map::new(),
                    });
                } else if let Some((target_file_slug, is_relative, original_name)) =
                    import_map.get(type_name.as_str())
                {
                    // Cross-file type via import — only for relative imports
                    // Use original_name (the exported name) not the local alias
                    if *is_relative {
                        let target_slug = format!("{target_file_slug}::{original_name}");
                        entities.push(GraphEntity::Edge {
                            edge_type: "TypeRef".to_string(),
                            from_slug: caller_slug.clone(),
                            to_slug: target_slug,
                            properties: Map::new(),
                        });
                    }
                }
            }
        }
    }
}

/// Extract type names from a function signature string.
///
/// Finds `: TypeName` patterns (after param names and before return type).
/// Excludes primitive types (string, number, boolean, void, any, etc.).
fn extract_type_names_from_signature(signature: &str, out: &mut HashSet<String>) {
    let primitives: HashSet<&str> = [
        "string", "number", "boolean", "void", "any", "never", "null",
        "undefined", "unknown", "object", "bigint", "symbol",
    ]
    .into_iter()
    .collect();

    let bytes = signature.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b':' {
            i += 1;
            // Skip whitespace after colon
            while i < len && bytes[i] == b' ' {
                i += 1;
            }
            // Read identifier
            if i < len && is_ident_start(bytes[i]) {
                let start = i;
                while i < len && is_ident_char(bytes[i]) {
                    i += 1;
                }
                let name = &signature[start..i];
                // Skip primitives and Promise wrapper
                if !primitives.contains(name) && name != "Promise" {
                    out.insert(name.to_string());
                }
            }
            continue;
        }
        i += 1;
    }
}

/// Extract imported identifier names from an import statement.
///
/// Returns `(local_name, original_name)` pairs.
/// - `import { a, b } from '...'` → [("a", "a"), ("b", "b")]
/// - `import { a as b } from '...'` → [("b", "a")] — local alias "b", original export "a"
/// - `import type { X } from '...'` → [("X", "X")]
fn extract_imported_names(import_text: &str) -> Vec<(String, String)> {
    let mut names = Vec::new();

    // Find the { ... } block
    if let Some(open) = import_text.find('{') {
        if let Some(close) = import_text.find('}') {
            let inner = &import_text[open + 1..close];
            for part in inner.split(',') {
                let trimmed = part.trim();
                if trimmed.is_empty() {
                    continue;
                }
                // Handle `name as alias` — track both original and local name
                if let Some(as_pos) = trimmed.find(" as ") {
                    let original = trimmed[..as_pos].trim();
                    let alias = trimmed[as_pos + 4..].trim();
                    if !alias.is_empty() && !original.is_empty() {
                        names.push((alias.to_string(), original.to_string()));
                    }
                } else {
                    names.push((trimmed.to_string(), trimmed.to_string()));
                }
            }
        }
    }
    names
}

/// Extract type name from matched text given the type kind keyword.
fn extract_type_name(text: &str, type_kind: &str) -> String {
    let keyword = match type_kind {
        "interface" => "interface ",
        "type_alias" => "type ",
        "enum_kind" => "enum ",
        "class" => "class ",
        _ => return String::new(),
    };
    helpers::extract_name_after_keyword(text, keyword)
}

/// Extract function name from matched text.
///
/// Handles both `function login(...)` and `async function login(...)`.
fn extract_function_name(text: &str) -> String {
    super::helpers::extract_name_after_keyword(text, "function ")
}

/// Extract the import path string from an import statement.
fn extract_import_path(text: &str) -> String {
    if let Some(from_pos) = text.find("from ") {
        let after = &text[from_pos + 5..];
        let quote_char = after.chars().next().unwrap_or(' ');
        if quote_char == '\'' || quote_char == '"' {
            let inner = &after[1..];
            if let Some(end) = inner.find(quote_char) {
                return inner[..end].to_string();
            }
        }
    }
    String::new()
}

/// Resolve a relative import path to a file path.
///
/// When `known_files` is non-empty, barrel imports are resolved: if the
/// initial resolution (e.g. `src/types.ts`) is not in `known_files`, the
/// resolver tries `src/types/index.{ts,tsx,js,jsx}` as fallbacks. This
/// correctly handles `import { Foo } from '../types'` when the actual
/// file is `src/types/index.ts`.
fn resolve_import_path(
    source_file: &str,
    import_path: &str,
    known_files: &HashSet<String>,
) -> String {
    if !import_path.starts_with('.') {
        return import_path.to_string();
    }

    let source_dir = if let Some(pos) = source_file.rfind('/') {
        &source_file[..pos]
    } else {
        ""
    };

    let mut parts: Vec<&str> = if source_dir.is_empty() {
        vec![]
    } else {
        source_dir.split('/').collect()
    };

    for segment in import_path.split('/') {
        match segment {
            "." => {}
            ".." => {
                parts.pop();
            }
            _ => parts.push(segment),
        }
    }

    let base = parts.join("/");

    // If the import already has a file extension, return as-is
    if base.ends_with(".ts")
        || base.ends_with(".tsx")
        || base.ends_with(".js")
        || base.ends_with(".jsx")
    {
        return base;
    }

    // Default resolution: append .ts
    let direct = format!("{base}.ts");

    // If we have no known files set, return the direct resolution (legacy behavior)
    if known_files.is_empty() {
        return direct;
    }

    // If the direct resolution exists in known files, use it
    if known_files.contains(&direct) {
        return direct;
    }

    // Try barrel import resolution: directory/index.{ts,tsx,js,jsx}
    for ext in &["ts", "tsx", "js", "jsx"] {
        let barrel = format!("{base}/index.{ext}");
        if known_files.contains(&barrel) {
            return barrel;
        }
    }

    // Also try .tsx fallback for the direct path (e.g. Component.tsx)
    for ext in &["tsx", "js", "jsx"] {
        let alt = format!("{base}.{ext}");
        if known_files.contains(&alt) {
            return alt;
        }
    }

    // No match found — return the default .ts resolution
    direct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_calls_finds_local_calls() {
        let source = r#"
function validateCommand(file) {
    const files = file ? [file] : findAllFeatureFiles();
    validateFile(files[0]);
}

function findAllFeatureFiles() {
    return [];
}

function validateFile(f) {
    checkForCommonIssues(f);
}

function checkForCommonIssues(content) {
    return getSuggestion(content);
}

function getSuggestion(msg) {
    return undefined;
}
"#;
        let entities = extract_typescript(source, "test/validate.ts", &HashSet::new()).unwrap();
        let calls: Vec<_> = entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Calls"))
            .collect();
        println!("Total entities: {}", entities.len());
        println!("Calls edges: {}", calls.len());
        for c in &calls {
            if let GraphEntity::Edge { from_slug, to_slug, .. } = c {
                println!("  {} -> {}", from_slug, to_slug);
            }
        }
        assert!(
            !calls.is_empty(),
            "Expected Calls edges for local function calls"
        );
    }

    #[test]
    fn test_extract_call_names_from_body_basic() {
        let body = r#"{ foo(); bar(x, y); baz.method(); new Constructor(); }"#;
        let mut names = HashSet::new();
        extract_call_names_from_body(body, &mut names);
        println!("Found call names: {:?}", names);
        assert!(names.contains("foo"), "Should find 'foo' call");
        assert!(names.contains("bar"), "Should find 'bar' call");
        assert!(!names.contains("method"), "Should NOT find method call");
        assert!(
            !names.contains("Constructor"),
            "Should NOT find constructor"
        );
    }

    #[test]
    fn test_extract_typescript_with_async_functions() {
        let source = r#"
export async function main() {
    const result = await helper();
    return result;
}

async function helper() {
    return 42;
}
"#;
        let entities = extract_typescript(source, "test/async.ts", &HashSet::new()).unwrap();
        let functions: Vec<_> = entities
            .iter()
            .filter(|e| {
                matches!(e, GraphEntity::Node { node_type, .. } if node_type == "Function")
            })
            .collect();
        let calls: Vec<_> = entities
            .iter()
            .filter(|e| matches!(e, GraphEntity::Edge { edge_type, .. } if edge_type == "Calls"))
            .collect();
        println!("Functions found: {}", functions.len());
        println!("Calls found: {}", calls.len());
        for f in &functions {
            if let GraphEntity::Node { slug, .. } = f {
                println!("  Function: {}", slug);
            }
        }
        for c in &calls {
            if let GraphEntity::Edge { from_slug, to_slug, .. } = c {
                println!("  Call: {} -> {}", from_slug, to_slug);
            }
        }
        assert_eq!(functions.len(), 2, "Should find 2 functions");
        assert!(
            !calls.is_empty(),
            "main() calls helper(), should have Calls edge"
        );
    }

    #[test]
    fn test_extract_imported_names_with_aliases() {
        // Without alias
        let names = extract_imported_names("import { foo, bar } from './mod'");
        assert_eq!(names, vec![
            ("foo".to_string(), "foo".to_string()),
            ("bar".to_string(), "bar".to_string()),
        ]);

        // With alias — local name "util", original name "createCheckpoint"
        let names = extract_imported_names(
            "import { createCheckpoint as util } from '../utils/git-checkpoint'"
        );
        assert_eq!(names, vec![
            ("util".to_string(), "createCheckpoint".to_string()),
        ]);

        // Mixed
        let names = extract_imported_names(
            "import { alpha, beta as b, gamma } from './lib'"
        );
        assert_eq!(names, vec![
            ("alpha".to_string(), "alpha".to_string()),
            ("b".to_string(), "beta".to_string()),
            ("gamma".to_string(), "gamma".to_string()),
        ]);
    }

    #[test]
    fn test_cross_file_calls_use_original_name_not_alias() {
        let source = r#"
import { createCheckpoint as createCheckpointUtil } from '../utils/git-checkpoint';

export function runCheckpoint() {
    const result = createCheckpointUtil({ workUnit: 'TEST-001' });
    return result;
}
"#;
        let entities = extract_typescript(source, "src/commands/checkpoint.ts", &HashSet::new()).unwrap();
        let calls: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                if let GraphEntity::Edge { edge_type, from_slug, to_slug, .. } = e {
                    if edge_type == "Calls" {
                        return Some((from_slug.as_str(), to_slug.as_str()));
                    }
                }
                None
            })
            .collect();
        println!("Calls edges: {:?}", calls);
        // Should resolve to the original name "createCheckpoint", not the alias
        assert!(
            calls.iter().any(|(_, to)| to.contains("::createCheckpoint") && !to.contains("::createCheckpointUtil")),
            "Cross-file call should use original export name, not alias. Got: {:?}",
            calls
        );
    }

    #[test]
    fn test_resolve_import_path_barrel_import() {
        // Without known_files, barrel import resolves to foo.ts (legacy)
        let empty = HashSet::new();
        let resolved = resolve_import_path("src/commands/program.ts", "../types", &empty);
        assert_eq!(resolved, "src/types.ts", "Without known_files, should resolve to .ts");

        // With known_files containing the index.ts barrel file
        let mut known = HashSet::new();
        known.insert("src/types/index.ts".to_string());
        let resolved = resolve_import_path("src/commands/program.ts", "../types", &known);
        assert_eq!(
            resolved, "src/types/index.ts",
            "With known_files, should resolve to index.ts barrel"
        );

        // Direct file still wins when it exists
        let mut known2 = HashSet::new();
        known2.insert("src/utils.ts".to_string());
        known2.insert("src/utils/index.ts".to_string());
        let resolved = resolve_import_path("src/commands/program.ts", "../utils", &known2);
        assert_eq!(
            resolved, "src/utils.ts",
            "Direct .ts file should win over barrel index.ts when both exist"
        );

        // Barrel import for schedule directory
        let mut known3 = HashSet::new();
        known3.insert("src/commands/schedule/index.ts".to_string());
        let resolved = resolve_import_path("src/cli/program.ts", "../commands/schedule", &known3);
        assert_eq!(
            resolved, "src/commands/schedule/index.ts",
            "Schedule barrel import should resolve to index.ts"
        );
    }

    #[test]
    fn test_resolve_import_path_tsx_fallback() {
        // TSX fallback when .ts doesn't exist
        let mut known = HashSet::new();
        known.insert("src/components/App.tsx".to_string());
        let resolved = resolve_import_path("src/index.ts", "./components/App", &known);
        assert_eq!(
            resolved, "src/components/App.tsx",
            "Should fall back to .tsx when .ts doesn't exist"
        );
    }

    #[test]
    fn test_extract_typescript_barrel_import_creates_correct_target() {
        let source = r#"
import { WorkUnitsData } from '../types';

export function listUnits(): void {
    return;
}
"#;
        let mut known = HashSet::new();
        known.insert("src/types/index.ts".to_string());

        let entities = extract_typescript(source, "src/commands/list.ts", &known).unwrap();

        // Find the Imports edge
        let import_edges: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                if let GraphEntity::Edge { edge_type, to_slug, .. } = e {
                    if edge_type == "Imports" {
                        return Some(to_slug.as_str());
                    }
                }
                None
            })
            .collect();

        assert_eq!(import_edges.len(), 1);
        assert_eq!(
            import_edges[0], "src-types-index-ts",
            "Import edge should target the barrel index.ts, not src-types-ts"
        );

        // Find the stub File node for the import target
        let target_file: Vec<_> = entities
            .iter()
            .filter_map(|e| {
                if let GraphEntity::Node { node_type, properties, .. } = e {
                    if node_type == "File" {
                        let path = properties.get("path").and_then(|v| v.as_str());
                        if path == Some("src/types/index.ts") {
                            return Some(path);
                        }
                    }
                }
                None
            })
            .collect();
        assert_eq!(
            target_file.len(), 1,
            "Should create stub File node with correct barrel path"
        );
    }
}
