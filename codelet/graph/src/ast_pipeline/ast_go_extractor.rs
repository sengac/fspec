//! Go AST Extractor
//!
//! Extracts Function nodes, Type nodes, File nodes, and relationship edges
//! (Contains, ContainsType, Imports, Calls) from Go source files using
//! ast-grep pattern matching.
//!
//! Go import resolution: string paths in import declarations.
//! Local imports (starting with `.` or matching known project files) produce
//! Imports edges. External packages (github.com/*, stdlib) are filtered.

use std::collections::{HashMap, HashSet};

use ast_grep_core::matcher::KindMatcher;
use ast_grep_language::{LanguageExt, SupportLang};

use super::complexity;
use super::metadata;
use super::variables;
use super::edge_helpers;
use super::helpers;
use crate::graph::graph_entities::GraphEntity;

/// AST node kinds for Go functions and methods.
const GO_FUNC_KINDS: &[&str] = &["function_declaration", "method_declaration"];

/// ast-grep patterns for Go type declarations.
const GO_TYPE_PATTERNS: &[(&str, &str)] = &[
    ("type $NAME struct { $$$FIELDS }", "struct_kind"),
    ("type $NAME interface { $$$METHODS }", "interface"),
];

/// Extract entities from Go source code.
///
/// Extracts File, Function, and Type nodes, plus Imports and Calls edges.
/// The `known_files` set is used for import resolution.
pub fn extract_go(
    source: &str,
    rel_path: &str,
    known_files: &HashSet<String>,
) -> Result<Vec<GraphEntity>, String> {
    let lang = SupportLang::Go;
    let file_slug = helpers::slugify_path(rel_path);
    let mut entities = Vec::new();

    let line_count = source.lines().count() as i32;
    let is_test = rel_path.ends_with("_test.go");

    entities.push(helpers::build_file_node(
        rel_path, &file_slug, "go", line_count, is_test,
    ));

    let root = lang.ast_grep(source);

    // Extract function declarations → collect names
    let function_names = extract_functions(&root, &file_slug, &mut entities);

    // Extract type declarations → collect names for TypeRef
    let type_names = extract_types(&root, &file_slug, &mut entities);

    // Extract imports → Imports edges + import map
    let import_map = extract_imports(source, &file_slug, known_files, &mut entities);

    // Add same-package Imports edges to other Go files in the same package
    add_same_package_edges(source, rel_path, &file_slug, known_files, &mut entities);

    // Extract Calls edges
    extract_calls(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

    // Extract TypeRef edges from function/method signatures
    extract_type_refs(source, &file_slug, &function_names, &type_names, &import_map, &mut entities);

    // Extract module-level variables
    variables::extract_variables(source, &file_slug, rel_path, "go", &mut entities);
    Ok(entities)
}

/// Extract function/method declarations from Go source using kind-based matching.
///
/// Uses `KindMatcher` for `function_declaration` and `method_declaration`
/// to capture both package-level functions AND method receivers.
///
/// Returns the set of function names found in this file.
fn extract_functions(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();
    let lang = SupportLang::Go;

    for kind_name in GO_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let matched_text = node.text();
            let name = extract_go_func_name(&matched_text);
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let start_pos = node.start_pos();
            let end_pos = node.end_pos();
            let is_public = name.starts_with(|c: char| c.is_uppercase());
            let param_count = helpers::count_params_go(&matched_text);

            let fn_slug = format!("{file_slug}::{name}");
            let cc = complexity::calculate(&matched_text, "go");
            let meta = metadata::extract_function_meta(&matched_text, "go");
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
                "go",
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

/// Extract the function/method name from Go source text.
///
/// Handles both:
/// - `func Name(...)` → extracts "Name"
/// - `func (r *Recv) Name(...)` → skips receiver, extracts "Name"
fn extract_go_func_name(text: &str) -> String {
    let after_func = text.strip_prefix("func ").unwrap_or(text);

    // If it starts with `(`, there's a receiver — skip past `) `
    if after_func.starts_with('(') {
        if let Some(close_paren) = after_func.find(") ") {
            let after_recv = &after_func[close_paren + 2..];
            return after_recv
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
        }
        return String::new();
    }

    // Regular function: first word after "func "
    after_func
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

/// Extract type declarations from Go source.
///
/// Returns the set of type names found in this file.
fn extract_types(
    root: &ast_grep_core::AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    file_slug: &str,
    entities: &mut Vec<GraphEntity>,
) -> HashSet<String> {
    let mut seen_names = HashSet::new();

    for (pattern, type_kind) in GO_TYPE_PATTERNS {
        for node in root.root().find_all(*pattern) {
            let matched_text = node.text();
            let name = helpers::extract_name_after_keyword(&matched_text, "type ");
            if name.is_empty() || !seen_names.insert(name.clone()) {
                continue;
            }

            let is_public = name.starts_with(|c: char| c.is_uppercase());

            let type_slug = format!("{file_slug}::{name}");
            let type_start = node.start_pos();
            let type_end = node.end_pos();
            let type_meta = metadata::extract_type_meta(&matched_text, "go");
            entities.push(helpers::build_type_node(
                file_slug, &name, type_kind, is_public,
                type_start.line() as i32 + 1, type_end.line() as i32 + 1,
                &type_meta.source, &type_meta.docstring, &type_meta.decorators,
                "go", type_meta.truncated,
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

/// Add implicit Imports edges between Go files in the same package.
///
/// Go files in the same directory sharing the same `package X` declaration
/// have implicit visibility to each other's symbols. We create bidirectional
/// Imports edges to represent this.
fn add_same_package_edges(
    source: &str,
    rel_path: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) {
    // Extract package name from first `package X` line
    let pkg_name = source
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with("package ") {
                Some(trimmed.strip_prefix("package ")?.trim().to_string())
            } else {
                None
            }
        });

    let pkg_name = match pkg_name {
        Some(n) => n,
        None => return,
    };

    // Get the directory of this file
    let dir = if let Some(slash_pos) = rel_path.rfind('/') {
        &rel_path[..slash_pos]
    } else {
        ""
    };

    // Find all other .go files in the same directory (same package)
    for known_file in known_files {
        if known_file == rel_path {
            continue;
        }
        if !known_file.ends_with(".go") {
            continue;
        }

        // Check if in same directory
        let other_dir = if let Some(slash_pos) = known_file.rfind('/') {
            &known_file[..slash_pos]
        } else {
            ""
        };

        if other_dir == dir {
            // Same directory → same package (we trust the convention)
            // Note: the reverse edge is created when the other file is extracted
            edge_helpers::build_import_edge(
                file_slug,
                &pkg_name,
                known_file,
                false,
                entities,
            );
        }
    }
}

/// Extract Go import statements and produce Imports edges.
///
/// Handles single imports (`import "path"`) and grouped imports (`import (...)`).
/// Local imports (starting with `.` or matching known project directories) produce
/// Imports edges. External packages are skipped.
fn extract_imports(
    source: &str,
    file_slug: &str,
    known_files: &HashSet<String>,
    entities: &mut Vec<GraphEntity>,
) -> HashMap<String, (String, bool, String)> {
    let mut import_map = HashMap::new();

    for line in source.lines() {
        let trimmed = line.trim();

        // Match import "path" or just "path" inside import block
        let import_path = if trimmed.starts_with("import \"") {
            // Single import: import "path"
            extract_go_import_path(trimmed)
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // Inside import(...) block
            trimmed.trim_matches('"').to_string()
        } else {
            continue;
        };

        if import_path.is_empty() {
            continue;
        }

        // Determine if this is a local import
        let is_local = import_path.starts_with('.')
            || is_go_local_import(&import_path, known_files);

        if is_local {
            // Resolve local import path
            let resolved = if import_path.starts_with('.') {
                // Relative: ./internal/util → internal/util (as directory)
                let clean = import_path.trim_start_matches("./");
                // Find any .go file in this directory from known_files
                let dir_prefix = format!("{clean}/");
                if let Some(first_file) = known_files.iter().find(|f| f.starts_with(&dir_prefix)) {
                    first_file.clone()
                } else {
                    format!("{clean}.go")
                }
            } else {
                format!("{import_path}.go")
            };

            // Populate import_map with the package name for cross-file call resolution
            let pkg_name = import_path.rsplit('/').next().unwrap_or(&import_path);
            let target_slug = helpers::slugify_path(&resolved);
            import_map.insert(
                pkg_name.to_string(),
                (target_slug, true, pkg_name.to_string()),
            );

            edge_helpers::build_import_edge(
                file_slug,
                &import_path,
                &resolved,
                false,
                entities,
            );
        }
    }
    import_map
}

/// Check if a Go import path is local (not external package or stdlib).
///
/// A local import doesn't contain a domain name (no dots in first segment).
fn is_go_local_import(import_path: &str, _known_files: &HashSet<String>) -> bool {
    // External packages typically have domain: github.com/..., golang.org/...
    // Stdlib packages are single words: fmt, os, strings, etc.
    // Local packages start with ./ or ../ or are relative without dots
    !import_path.contains('.')
        && !is_go_stdlib_package(import_path)
}

/// Check if an import path is a Go standard library package.
fn is_go_stdlib_package(path: &str) -> bool {
    const GO_STDLIB: &[&str] = &[
        "fmt", "os", "io", "net", "http", "strings", "bytes", "bufio",
        "encoding", "crypto", "sync", "context", "errors", "flag",
        "log", "math", "path", "reflect", "regexp", "runtime", "sort",
        "strconv", "testing", "time", "unicode",
    ];
    let first_segment = path.split('/').next().unwrap_or(path);
    GO_STDLIB.contains(&first_segment)
}

/// Extract import path from a single Go import statement.
fn extract_go_import_path(line: &str) -> String {
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    String::new()
}

/// Extract Calls edges from Go function bodies.
fn extract_calls(
    source: &str,
    file_slug: &str,
    local_functions: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Go;
    let root = lang.ast_grep(source);

    for kind_name in GO_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_go_func_name(&fn_text);

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

/// Extract TypeRef edges from Go function/method signatures.
///
/// Go type references appear in:
/// - Function parameters: `func Foo(c *Command, name string)`
/// - Return types: `func Foo() (*Command, error)`
/// - Method receivers: `func (c *Command) Foo()`
///
/// Filters out Go builtin types.
fn extract_type_refs(
    source: &str,
    file_slug: &str,
    function_names: &HashSet<String>,
    local_types: &HashSet<String>,
    import_map: &HashMap<String, (String, bool, String)>,
    entities: &mut Vec<GraphEntity>,
) {
    let lang = SupportLang::Go;
    let root = lang.ast_grep(source);

    let go_builtins: HashSet<&str> = [
        "string", "int", "int8", "int16", "int32", "int64",
        "uint", "uint8", "uint16", "uint32", "uint64", "uintptr",
        "float32", "float64", "complex64", "complex128",
        "bool", "byte", "rune", "error", "any",
    ]
    .into_iter()
    .collect();

    for kind_name in GO_FUNC_KINDS {
        let matcher = match KindMatcher::try_new(kind_name, lang) {
            Ok(m) => m,
            Err(_) => continue,
        };

        for node in root.root().find_all(matcher.clone()) {
            let fn_text = node.text();
            let fn_name = extract_go_func_name(&fn_text);

            if fn_name.is_empty() || !function_names.contains(&fn_name) {
                continue;
            }

            let fn_slug = format!("{file_slug}::{fn_name}");

            // Get the signature (everything before the first `{`)
            let signature = if let Some(brace_pos) = fn_text.find('{') {
                &fn_text[..brace_pos]
            } else {
                &fn_text
            };

            let mut type_names = HashSet::new();
            extract_go_type_annotations(signature, &go_builtins, &mut type_names);

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

/// Extract type names from Go function signatures.
///
/// Looks for capitalized identifiers after `*` or in parameter/return positions.
/// Go convention: types start with uppercase letter.
fn extract_go_type_annotations(
    signature: &str,
    builtins: &HashSet<&str>,
    out: &mut HashSet<String>,
) {
    // Find all words that look like Go type names (capitalized, after * or space)
    let bytes = signature.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        // Look for `*TypeName` or standalone `TypeName` in parameter positions
        if bytes[i] == b'*' {
            i += 1;
            if i < len && bytes[i].is_ascii_uppercase() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let type_name = &signature[start..i];
                if !builtins.contains(type_name) {
                    out.insert(type_name.to_string());
                }
                continue;
            }
        }

        // Look for capitalized words that could be types in parameter lists
        // Context: after `,` or `(` followed by whitespace, or after type keyword
        if bytes[i].is_ascii_uppercase() {
            let start = i;
            while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &signature[start..i];
            // Only include if it looks like a type (not a function name or keyword)
            // Types in Go are followed by `,` `)` `{` or another word (var name)
            if !builtins.contains(word) && word != "func" {
                out.insert(word.to_string());
            }
            continue;
        }

        i += 1;
    }
}
