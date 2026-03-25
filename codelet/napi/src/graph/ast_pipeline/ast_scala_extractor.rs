//! Scala AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls, TypeRef) from Scala source files
//! using ast-grep pattern matching.
//!
//! Scala import resolution: `import com.package.Class` → `com/package/Class.scala`.
//! External imports (`scala.*`, `java.*`) are filtered out.

use std::collections::{HashMap, HashSet};

use ast_grep_language::{LanguageExt, SupportLang};

use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// ast-grep patterns for Scala function declarations.
const SCALA_FUNCTION_PATTERNS: &[&str] = &[
    "def $NAME($$$ARGS): $RET = { $$$BODY }",
    "def $NAME($$$ARGS) = { $$$BODY }",
    "def $NAME($$$ARGS): $RET = $BODY",
    "def $NAME($$$ARGS) = $BODY",
    "def $NAME($$$ARGS) { $$$BODY }",
];

/// ast-grep patterns for Scala type declarations.
const SCALA_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("class $NAME { $$$BODY }", "class"),
    ("class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("case class $NAME($$$ARGS) { $$$BODY }", "class"),
    ("case class $NAME($$$ARGS)", "class"),
    ("trait $NAME { $$$BODY }", "trait_kind"),
    ("object $NAME { $$$BODY }", "class"),
];

/// Scala standard library / Java package prefixes that should NOT produce Imports edges.
const SCALA_EXTERNAL_PREFIXES: &[&str] = &[
    "scala.", "java.", "javax.", "akka.", "org.apache", "org.scalatest",
    "org.specs2", "play.", "cats.", "zio.",
];

/// Scala built-in types to filter from TypeRef edges.
const SCALA_BUILTIN_TYPES: &[&str] = &[
    "Int", "Long", "Short", "Byte", "Float", "Double", "Boolean", "Char",
    "String", "Unit", "Any", "AnyRef", "AnyVal", "Nothing", "Null",
    "Option", "Some", "None", "List", "Map", "Set", "Seq", "Vector",
    "Array", "Tuple", "Either", "Left", "Right", "Future", "Try",
    "Success", "Failure", "Iterable", "Iterator", "Comparable",
    "Throwable", "Exception", "Void", "Object", "BigInt", "BigDecimal",
];

/// Extract entities from Scala source code.
///
/// Extracts File, Function, and Type nodes, plus Imports, Calls, and TypeRef edges.
pub fn extract_scala(source: &str, rel_path: &str, known_files: &HashSet<String>) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Scala;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.contains("Test")
        || rel_path.contains("Spec")
        || rel_path.contains("test/")
        || rel_path.contains("spec/");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "scala", line_count, is_test,
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
        source, &file_slug, &function_names, &type_names, &import_map, &mut entities,
    );

    Ok(entities)
}

/// Extract function declarations from Scala source.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for pattern in SCALA_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "def ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("protected ");
            let param_count = helpers::count_params(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            entities.push(helpers::build_function_node(
                file_slug,
                &name,
                false,
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

/// Extract type declarations from Scala source.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in SCALA_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let keyword = if matched_text.contains("case class ") {
                "class "
            } else if matched_text.starts_with("object ") || matched_text.contains(" object ") {
                "object "
            } else if matched_text.contains("trait ") {
                "trait "
            } else {
                "class "
            };
            let name = helpers::extract_name_after_keyword(&matched_text, keyword);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = !matched_text.starts_with("private ")
                && !matched_text.starts_with("protected ");

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

/// Extract Scala `import` statements and produce Imports edges.
///
/// Resolves package paths to file paths: `com.myapp.service.UserService`
/// → `com/myapp/service/UserService.scala`. Only produces edges for imports
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

        // Skip wildcard imports like `import com.myapp._`
        if import_path.ends_with("._") {
            continue;
        }

        // Skip external/standard library imports
        if SCALA_EXTERNAL_PREFIXES.iter().any(|p| import_path.starts_with(p)) {
            continue;
        }

        // Get local name (last segment)
        let local_name = import_path
            .rsplit('.')
            .next()
            .unwrap_or(import_path)
            .to_string();

        // Resolve to file path: dots → slashes + .scala
        let resolved_path = import_path.replace('.', "/") + ".scala";
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

/// Extract Calls edges from Scala function bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Scala;
    let root = lang.ast_grep(source);

    for pattern in SCALA_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = helpers::extract_name_after_keyword(&fn_text, "def ");
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

/// Extract TypeRef edges from Scala function signatures.
///
/// Scala signatures use `: Type` for parameter types and `: ReturnType` after `)`.
/// Example: `def handle(req: Request): Response`
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Scala;
    let root = lang.ast_grep(source);

    let builtins: HashSet<&str> = SCALA_BUILTIN_TYPES.iter().copied().collect();

    for pattern in SCALA_FUNCTION_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let fn_text = node.text();
            let fn_name = helpers::extract_name_after_keyword(&fn_text, "def ");
            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let fn_slug = format!("{file_slug}::{fn_name}");

            let signature = if let Some(brace_pos) = fn_text.find('{') {
                &fn_text[..brace_pos]
            } else if let Some(eq_pos) = fn_text.find('=') {
                &fn_text[..eq_pos]
            } else {
                &fn_text
            };

            let mut type_names = HashSet::new();
            extract_scala_type_annotations(signature, &builtins, &mut type_names);

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

/// Extract type names from Scala function signatures.
///
/// Scala type annotations appear after `:` characters:
/// - Parameter types: `def handle(req: Request)`
/// - Return type: `def handle(): Response`
///
/// Filters out Scala built-in types.
fn extract_scala_type_annotations(
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
